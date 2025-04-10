use core::panic;
use std::{path::PathBuf, sync::Arc};

use amaru_consensus::{consensus::{chain_selection::{ChainSelector, ChainSelectorBuilder}, header_validation::Consensus, store::ChainStore}, peer::Peer, ConsensusError, RawHeader};
use amaru_kernel::{network::{EraHistory, NetworkName}, protocol_parameters::ProtocolParameters, Hash, Hasher, Header, Point};
use amaru_ledger::{context, rules::{self, parse_block}, state::State};
use amaru_stores::rocksdb::{consensus::RocksDBStore, RocksDB};
use pallas_network::{facades::PeerClient, miniprotocols::chainsync::{HeaderContent, NextResponse}};
use pallas_traverse::MultiEraHeader;
use tokio::sync::Mutex;

/*pub async fn find_intersection(peer: &Peer) -> () {
    let client = peer.chainsync();
    let (point, _) = client
        .find_intersect(
            intersection
                .iter()
                .cloned()
                .map(to_network_point)
                .collect(),
        )
        .await;
}

async fn next_block(peer: &PeerClient) -> NextResponse<HeaderContent> {
    let client = peer.chainsync();

    if client.has_agency() {
        // should request next block
        client.request_next().await?
    } else {
        // should await for next block
        match timeout(Duration::from_secs(1), client.recv_while_must_reply()).await {
            Ok(result) => result?,
            Err(_) => Err(WorkerError::Retry)?,
        }
    }
}*/

pub async fn bootstrap(peer_address: &String) -> Result<(), Box<dyn std::error::Error>> {
    let network = NetworkName::Preprod;
    let ledger_dir = PathBuf::from("./ledger.db");
    let chain_dir = PathBuf::from("./chain.db");

    let era_history: &EraHistory = network.into();
    let store = RocksDB::new(&ledger_dir, era_history)?;
    let mut state = State::new(Arc::new(std::sync::Mutex::new(store)), era_history);
    let tip = state.tip().into_owned();

    let mut peer_client = PeerClient::connect(peer_address.clone(), network.to_network_magic() as u64).await?;
    let peer = Peer::new(&peer_address);

    let chain_store = RocksDBStore::new(chain_dir.clone(), era_history)?;
    let chain_selector = make_chain_selector(tip, &chain_store, &vec![peer.clone()])?;
    let chain_ref = Arc::new(Mutex::new(chain_store));
    let mut consensus = Consensus::new(
        Box::new(state.view_stake_distribution()),
        chain_ref.clone(),
        chain_selector,
    );

    let next_response = peer_client.chainsync().request_next().await.unwrap_or_else(|_| panic!("Failed to get response"));
    let header = match next_response {
        NextResponse::RollForward(header, _) => header,
        _ => panic!("Received wrong response {:?}", next_response),
    };

    let header = to_traverse(&header);
    let point = Point::Specific(header.slot(), header.hash().to_vec());

    let raw_header: RawHeader = header.cbor().to_vec();

    let events= consensus
            .handle_roll_forward(&peer, &point, &raw_header)
            .await?;

    // TODO handle consensus logic

    let raw_block = fetch_block(peer_client, point.clone()).await?;

    let mut ctx = context::DefaultPreparationContext::new();

    let block = parse_block(&raw_block[..])?;

    let issuer = Hasher::<224>::hash(&block.header.header_body.issuer_vkey[..]);

    rules::prepare_block(&mut ctx, &block);

    // TODO: Eventually move into a separate function, or integrate within the ledger instead
    // of the current .resolve_inputs; once the latter is no longer needed for the state
    // construction.
    let inputs = state
        .resolve_inputs(&Default::default(), ctx.utxo.into_iter())?
        .into_iter()
        // NOTE:
        // It isn't okay to just fail early here because we may be missing UTxO even on valid
        // transactions! Indeed, since we only have access to the _current_ volatile DB and the
        // immutable DB. That means, we can't be aware of UTxO created and used within the block.
        //
        // Those will however be produced during the validation, and be tracked by the
        // validation context.
        //
        // Hence, we *must* defer errors here until the moment we do expect the UTxO to be
        // present.
        .filter_map(|(input, opt_output)| opt_output.map(|output| (input, output)))
        .collect();

    let volatile_state = rules::validate_block(
        context::DefaultValidationContext::new(inputs),
        ProtocolParameters::default(),
        block,
    )?;

    state.forward(volatile_state.anchor(&point, issuer))?;

    Ok(())
}

pub fn to_traverse(header: &HeaderContent) -> MultiEraHeader<'_> {
    match header.byron_prefix {
        Some((subtag, _)) => MultiEraHeader::decode(header.variant, Some(subtag), &header.cbor).unwrap(),
        None => MultiEraHeader::decode(header.variant, None, &header.cbor).unwrap(),
    }
}

async fn fetch_block(mut peer_client: PeerClient, point: Point) -> Result<Vec<u8>, ConsensusError> {
    let client = peer_client.blockfetch();
    let new_point: pallas_network::miniprotocols::Point = match point.clone() {
        Point::Origin => pallas_network::miniprotocols::Point::Origin,
        Point::Specific(slot, hash) => {
            pallas_network::miniprotocols::Point::Specific(slot, hash)
        }
    };
    client
        .fetch_single(new_point)
        .await
        .map_err(|_| ConsensusError::FetchBlockFailed(point))
}

fn make_chain_selector(
    tip: Point,
    chain_store: &impl ChainStore<Header>,
    peers: &Vec<Peer>,
) -> Result<Arc<Mutex<ChainSelector<Header>>>, ConsensusError> {
    let mut builder = ChainSelectorBuilder::new();

    #[allow(clippy::panic)]
    match chain_store.load_header(&Hash::from(&tip)) {
        None => panic!("Tip {:?} not found in chain store", tip),
        Some(header) => builder.set_tip(&header),
    };

    for peer in peers {
        builder.add_peer(&peer);
    }

    Ok(Arc::new(Mutex::new(builder.build()?)))
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    bootstrap(&"127.0.0.1:3000".into()).await
}