mod certificates;
mod connections;
mod error;
mod scalars;

use self::connections::{
    BlockConnection, InclusivePaginationInterval, PaginationArguments, PaginationInterval,
    PoolConnection, TransactionConnection, TransactionNodeFetchInfo, VotePlanConnection,
    VoteStatusConnection,
};
use self::error::ErrorKind;
use self::scalars::{
    BlockCount, ChainLength, EpochNumber, ExternalProposalId, IndexCursor, NonZero, PayloadType,
    PoolId, PublicKey, Slot, Value, VoteOptionRange, VotePlanId, Weight,
};
use super::indexing::{
    BlockProducer, EpochData, ExplorerAddress, ExplorerBlock, ExplorerTransaction, StakePoolData,
};
use super::persistent_sequence::PersistentSequence;
use crate::blockcfg::{self, FragmentId, HeaderHash};
use crate::explorer::indexing::ExplorerVote;
use crate::explorer::{ExplorerDB, Settings};
use cardano_legacy_address::Addr as OldAddress;
use certificates::*;
use chain_impl_mockchain::certificate;
use chain_impl_mockchain::key::BftLeaderId;
use chain_impl_mockchain::vote::{EncryptedVote, ProofOfCorrectVote};
pub use juniper::http::GraphQLRequest;
use juniper::{EmptyMutation, EmptySubscription, FieldResult, GraphQLUnion, RootNode};
use std::convert::{TryFrom, TryInto};
use std::str::FromStr;
use std::sync::Arc;

#[derive(Clone)]
pub struct Block {
    hash: HeaderHash,
}

impl Block {
    async fn from_string_hash(hash: String, db: &ExplorerDB) -> FieldResult<Block> {
        let hash = HeaderHash::from_str(&hash)?;
        let block = Block { hash };

        block.get_explorer_block(db).await.map(|_| block)
    }

    fn from_valid_hash(hash: HeaderHash) -> Block {
        Block { hash }
    }

    async fn get_explorer_block(&self, db: &ExplorerDB) -> FieldResult<Arc<ExplorerBlock>> {
        db.get_block(&self.hash).await.ok_or_else(|| {
            ErrorKind::InternalError("Couldn't find block's contents in explorer".to_owned()).into()
        })
    }
}

/// A Block
#[juniper::graphql_object(
    Context = Context
)]
impl Block {
    /// The Block unique identifier
    pub fn id(&self) -> String {
        format!("{}", self.hash)
    }

    /// Date the Block was included in the blockchain
    pub async fn date(&self, context: &Context) -> FieldResult<BlockDate> {
        self.get_explorer_block(&context.db)
            .await
            .map(|b| b.date().into())
    }

    /// The transactions contained in the block
    pub async fn transactions(
        &self,
        first: Option<i32>,
        last: Option<i32>,
        before: Option<IndexCursor>,
        after: Option<IndexCursor>,
        context: &Context,
    ) -> FieldResult<TransactionConnection> {
        let explorer_block = self.get_explorer_block(&context.db).await?;
        let mut transactions: Vec<&ExplorerTransaction> =
            explorer_block.transactions.values().collect();

        // TODO: This may be expensive at some point, but I can't rely in
        // the HashMap's order (also, I'm assuming the order in the block matters)
        transactions
            .as_mut_slice()
            .sort_unstable_by_key(|tx| tx.offset_in_block);

        let pagination_arguments = PaginationArguments {
            first,
            last,
            before: before.map(u32::try_from).transpose()?,
            after: after.map(u32::try_from).transpose()?,
        }
        .validate()?;

        let boundaries = if !transactions.is_empty() {
            PaginationInterval::Inclusive(InclusivePaginationInterval {
                lower_bound: 0u32,
                upper_bound: transactions
                    .len()
                    .checked_sub(1)
                    .unwrap()
                    .try_into()
                    .expect("tried to paginate more than 2^32 elements"),
            })
        } else {
            PaginationInterval::Empty
        };

        TransactionConnection::new(
            boundaries,
            pagination_arguments,
            |range: PaginationInterval<u32>| match range {
                PaginationInterval::Empty => vec![],
                PaginationInterval::Inclusive(range) => {
                    let from = usize::try_from(range.lower_bound).unwrap();
                    let to = usize::try_from(range.upper_bound).unwrap();

                    (from..=to)
                        .map(|i| {
                            (
                                TransactionNodeFetchInfo::Contents(transactions[i].clone()),
                                i.try_into().unwrap(),
                            )
                        })
                        .collect::<Vec<(TransactionNodeFetchInfo, u32)>>()
                }
            },
        )
    }

    pub async fn previous_block(&self, context: &Context) -> FieldResult<Block> {
        self.get_explorer_block(&context.db)
            .await
            .map(|b| Block::from_valid_hash(b.parent_hash))
    }

    pub async fn chain_length(&self, context: &Context) -> FieldResult<ChainLength> {
        self.get_explorer_block(&context.db)
            .await
            .map(|block| block.chain_length().into())
    }

    pub async fn leader(&self, context: &Context) -> FieldResult<Option<Leader>> {
        self.get_explorer_block(&context.db)
            .await
            .map(|block| match block.producer() {
                BlockProducer::StakePool(pool) => {
                    Some(Leader::StakePool(Pool::from_valid_id(pool.clone())))
                }
                BlockProducer::BftLeader(id) => {
                    Some(Leader::BftLeader(BftLeader { id: id.clone() }))
                }
                BlockProducer::None => None,
            })
    }

    pub async fn total_input(&self, context: &Context) -> FieldResult<Value> {
        self.get_explorer_block(&context.db)
            .await
            .map(|block| Value(format!("{}", block.total_input)))
    }

    pub async fn total_output(&self, context: &Context) -> FieldResult<Value> {
        self.get_explorer_block(&context.db)
            .await
            .map(|block| Value(format!("{}", block.total_output)))
    }

    pub async fn treasury(&self, context: &Context) -> FieldResult<Option<Treasury>> {
        let treasury = context
            .db
            .blockchain()
            .get_ref(self.hash)
            .await
            .unwrap_or(None)
            .map(|reference| {
                let ledger = reference.ledger();
                let treasury_tax = reference.epoch_ledger_parameters().treasury_tax;
                Treasury {
                    rewards: ledger.remaining_rewards().into(),
                    treasury: ledger.treasury_value().into(),
                    treasury_tax: TaxType(treasury_tax),
                }
            });
        Ok(treasury)
    }

    pub async fn is_confirmed(&self, context: &Context) -> bool {
        context.db.is_block_confirmed(&self.hash).await
    }
}

struct BftLeader {
    id: BftLeaderId,
}

#[juniper::graphql_object(
    Context = Context,
)]
impl BftLeader {
    fn id(&self) -> PublicKey {
        self.id.as_public_key().into()
    }
}

#[derive(GraphQLUnion)]
#[graphql(Context = Context)]
enum Leader {
    StakePool(Pool),
    BftLeader(BftLeader),
}

impl From<Arc<ExplorerBlock>> for Block {
    fn from(block: Arc<ExplorerBlock>) -> Block {
        Block::from_valid_hash(block.id())
    }
}

#[derive(Clone)]
struct BlockDate {
    epoch: Epoch,
    slot: Slot,
}

/// Block's date, composed of an Epoch and a Slot
#[juniper::graphql_object(
    Context = Context
)]
impl BlockDate {
    pub fn epoch(&self) -> &Epoch {
        &self.epoch
    }

    pub fn slot(&self) -> &Slot {
        &self.slot
    }
}

impl From<blockcfg::BlockDate> for BlockDate {
    fn from(date: blockcfg::BlockDate) -> BlockDate {
        BlockDate {
            epoch: Epoch { id: date.epoch },
            slot: Slot(format!("{}", date.slot_id)),
        }
    }
}

#[derive(Clone)]
struct Transaction {
    id: FragmentId,
    block_hash: Option<HeaderHash>,
    contents: Option<ExplorerTransaction>,
}

impl Transaction {
    async fn from_id(id: FragmentId, context: &Context) -> FieldResult<Transaction> {
        let block_hash = context
            .db
            .get_main_tip()
            .await
            .1
            .state()
            .find_block_hash_by_transaction(&id)
            .ok_or_else(|| ErrorKind::NotFound(format!("transaction not found: {}", &id,)))?;

        Ok(Transaction {
            id,
            block_hash: Some(block_hash),
            contents: None,
        })
    }

    fn from_valid_id(id: FragmentId) -> Transaction {
        Transaction {
            id,
            block_hash: None,
            contents: None,
        }
    }

    fn from_contents(contents: ExplorerTransaction) -> Transaction {
        Transaction {
            id: contents.id,
            block_hash: None,
            contents: Some(contents),
        }
    }

    async fn get_block(&self, context: &Context) -> FieldResult<Arc<ExplorerBlock>> {
        let block_id = match self.block_hash {
            Some(block_id) => block_id,
            None => context
                .db
                .get_main_tip()
                .await
                .1
                .state()
                .find_block_hash_by_transaction(&self.id)
                .ok_or_else(|| {
                    ErrorKind::InternalError("Transaction's block was not found".to_owned())
                })?,
        };

        context.db.get_block(&block_id).await.ok_or_else(|| {
            ErrorKind::InternalError(
                "transaction is in explorer but couldn't find its block".to_owned(),
            )
            .into()
        })
    }

    async fn get_contents(&self, context: &Context) -> FieldResult<ExplorerTransaction> {
        if let Some(c) = &self.contents {
            Ok(c.clone())
        } else {
            let block = self.get_block(context).await?;
            Ok(block
                .transactions
                .get(&self.id)
                .ok_or_else(|| {
                    ErrorKind::InternalError(
                        "transaction was not found in respective block".to_owned(),
                    )
                })?
                .clone())
        }
    }
}

/// A transaction in the blockchain
#[juniper::graphql_object(
    Context = Context
)]
impl Transaction {
    /// The hash that identifies the transaction
    pub fn id(&self) -> String {
        format!("{}", self.id)
    }

    /// The block this transaction is in
    pub async fn block(&self, context: &Context) -> FieldResult<Block> {
        let block = self.get_block(context).await?;
        Ok(Block::from(block))
    }

    pub async fn inputs(&self, context: &Context) -> FieldResult<Vec<TransactionInput>> {
        let transaction = self.get_contents(context).await?;
        Ok(transaction
            .inputs()
            .iter()
            .map(|input| TransactionInput {
                address: Address::from(&input.address),
                amount: Value::from(&input.value),
            })
            .collect())
    }

    pub async fn outputs(&self, context: &Context) -> FieldResult<Vec<TransactionOutput>> {
        let transaction = self.get_contents(context).await?;
        Ok(transaction
            .outputs()
            .iter()
            .map(|input| TransactionOutput {
                address: Address::from(&input.address),
                amount: Value::from(&input.value),
            })
            .collect())
    }

    pub async fn certificate(
        &self,
        context: &Context,
    ) -> FieldResult<Option<certificates::Certificate>> {
        let transaction = self.get_contents(context).await?;
        match transaction.certificate {
            Some(c) => Certificate::try_from(c).map(Some).map_err(|e| e.into()),
            None => Ok(None),
        }
    }
}

struct TransactionInput {
    amount: Value,
    address: Address,
}

#[juniper::graphql_object(
    Context = Context
)]
impl TransactionInput {
    fn amount(&self) -> &Value {
        &self.amount
    }

    fn address(&self) -> &Address {
        &self.address
    }
}

struct TransactionOutput {
    amount: Value,
    address: Address,
}

#[juniper::graphql_object(
    Context = Context
)]
impl TransactionOutput {
    fn amount(&self) -> &Value {
        &self.amount
    }

    fn address(&self) -> &Address {
        &self.address
    }
}

#[derive(Clone)]
struct Address {
    id: ExplorerAddress,
}

impl Address {
    fn from_bech32(bech32: &str) -> FieldResult<Address> {
        let addr = chain_addr::AddressReadable::from_string_anyprefix(bech32)
            .map(|adr| ExplorerAddress::New(adr.to_address()))
            .or_else(|_| OldAddress::from_str(bech32).map(ExplorerAddress::Old))
            .map_err(|_| ErrorKind::InvalidAddress(bech32.to_string()))?;

        Ok(Address { id: addr })
    }
}

impl From<&ExplorerAddress> for Address {
    fn from(addr: &ExplorerAddress) -> Address {
        Address { id: addr.clone() }
    }
}

#[juniper::graphql_object(
    Context = Context
)]
impl Address {
    /// The base32 representation of an address
    fn id(&self, context: &Context) -> String {
        match &self.id {
            ExplorerAddress::New(addr) => chain_addr::AddressReadable::from_address(
                &context.settings.address_bech32_prefix,
                addr,
            )
            .to_string(),
            ExplorerAddress::Old(addr) => format!("{}", addr),
        }
    }

    fn delegation() -> FieldResult<Pool> {
        Err(ErrorKind::Unimplemented.into())
    }

    async fn transactions(
        &self,
        first: Option<i32>,
        last: Option<i32>,
        before: Option<IndexCursor>,
        after: Option<IndexCursor>,
        context: &Context,
    ) -> FieldResult<TransactionConnection> {
        let transactions = context
            .db
            .get_main_tip()
            .await
            .1
            .state()
            .transactions_by_address(&self.id)
            .unwrap_or_else(PersistentSequence::<FragmentId>::new);

        let boundaries = if transactions.len() > 0 {
            PaginationInterval::Inclusive(InclusivePaginationInterval {
                lower_bound: 0u64,
                upper_bound: transactions.len(),
            })
        } else {
            PaginationInterval::Empty
        };

        let pagination_arguments = PaginationArguments {
            first,
            last,
            before: before.map(u64::from),
            after: after.map(u64::from),
        }
        .validate()?;

        TransactionConnection::new(
            boundaries,
            pagination_arguments,
            |range: PaginationInterval<u64>| match range {
                PaginationInterval::Empty => vec![],
                PaginationInterval::Inclusive(range) => (range.lower_bound..=range.upper_bound)
                    .filter_map(|i| {
                        transactions
                            .get(i)
                            .map(|h| (TransactionNodeFetchInfo::Id(*h.as_ref()), i))
                    })
                    .collect(),
            },
        )
    }
}

struct TaxType(chain_impl_mockchain::rewards::TaxType);

#[juniper::graphql_object(
    Context = Context,
)]
impl TaxType {
    /// what get subtracted as fixed value
    pub fn fixed(&self) -> Value {
        Value(format!("{}", self.0.fixed))
    }
    /// Ratio of tax after fixed amout subtracted
    pub fn ratio(&self) -> Ratio {
        Ratio(self.0.ratio)
    }

    /// Max limit of tax
    pub fn max_limit(&self) -> Option<NonZero> {
        self.0.max_limit.map(|n| NonZero(format!("{}", n)))
    }
}

struct Ratio(chain_impl_mockchain::rewards::Ratio);

#[juniper::graphql_object(
    Context = Context,
)]
impl Ratio {
    pub fn numerator(&self) -> Value {
        Value(format!("{}", self.0.numerator))
    }

    pub fn denominator(&self) -> NonZero {
        NonZero(format!("{}", self.0.denominator))
    }
}

pub struct Proposal(certificate::Proposal);

#[juniper::graphql_object(
    Context = Context,
)]
impl Proposal {
    pub fn external_id(&self) -> ExternalProposalId {
        ExternalProposalId(self.0.external_id().to_string())
    }

    /// get the vote options range
    ///
    /// this is the available range of choices to make for the given
    /// proposal. all casted votes for this proposals ought to be in
    /// within the given range
    pub fn options(&self) -> VoteOptionRange {
        self.0.options().clone().into()
    }
}

#[derive(Clone)]
pub struct Pool {
    id: certificate::PoolId,
    data: Option<Arc<StakePoolData>>,
    blocks: Option<Arc<PersistentSequence<HeaderHash>>>,
}

impl Pool {
    async fn from_string_id(id: &str, db: &ExplorerDB) -> FieldResult<Pool> {
        let id = certificate::PoolId::from_str(&id)?;
        let blocks = db
            .get_stake_pool_blocks(&id)
            .await
            .ok_or_else(|| ErrorKind::NotFound("Stake pool not found".to_owned()))?;

        let data = db
            .get_stake_pool_data(&id)
            .await
            .ok_or_else(|| ErrorKind::NotFound("Stake pool not found".to_owned()))?;

        Ok(Pool {
            id,
            data: Some(data),
            blocks: Some(blocks),
        })
    }

    fn from_valid_id(id: certificate::PoolId) -> Pool {
        Pool {
            id,
            blocks: None,
            data: None,
        }
    }

    fn new_with_data(id: certificate::PoolId, data: Arc<StakePoolData>) -> Self {
        Pool {
            id,
            blocks: None,
            data: Some(data),
        }
    }
}

#[juniper::graphql_object(
    Context = Context
)]
impl Pool {
    pub fn id(&self) -> PoolId {
        PoolId(format!("{}", &self.id))
    }

    pub async fn blocks(
        &self,
        first: Option<i32>,
        last: Option<i32>,
        before: Option<IndexCursor>,
        after: Option<IndexCursor>,
        context: &Context,
    ) -> FieldResult<BlockConnection> {
        let blocks = match &self.blocks {
            Some(b) => b.clone(),
            None => context
                .db
                .get_stake_pool_blocks(&self.id)
                .await
                .ok_or_else(|| {
                    ErrorKind::InternalError("Stake pool in block is not indexed".to_owned())
                })?,
        };

        let bounds = if blocks.len() > 0 {
            PaginationInterval::Inclusive(InclusivePaginationInterval {
                lower_bound: 0u32,
                upper_bound: blocks
                    .len()
                    .checked_sub(1)
                    .unwrap()
                    .try_into()
                    .expect("Tried to paginate more than 2^32 blocks"),
            })
        } else {
            PaginationInterval::Empty
        };

        let pagination_arguments = PaginationArguments {
            first,
            last,
            before: before.map(u32::try_from).transpose()?,
            after: after.map(u32::try_from).transpose()?,
        }
        .validate()?;

        BlockConnection::new(bounds, pagination_arguments, |range| match range {
            PaginationInterval::Empty => vec![],
            PaginationInterval::Inclusive(range) => (range.lower_bound..=range.upper_bound)
                .filter_map(|i| blocks.get(i).map(|h| (*h.as_ref(), i)))
                .collect(),
        })
    }

    pub async fn registration(&self, context: &Context) -> FieldResult<PoolRegistration> {
        match &self.data {
            Some(data) => Ok(data.registration.clone().into()),
            None => context
                .db
                .get_stake_pool_data(&self.id)
                .await
                .map(|data| PoolRegistration::from(data.registration.clone()))
                .ok_or_else(|| ErrorKind::NotFound("Stake pool not found".to_owned()).into()),
        }
    }

    pub async fn retirement(&self, context: &Context) -> FieldResult<Option<PoolRetirement>> {
        match &self.data {
            Some(data) => Ok(data.retirement.clone().map(PoolRetirement::from)),
            None => context
                .db
                .get_stake_pool_data(&self.id)
                .await
                .ok_or_else(|| ErrorKind::NotFound("Stake pool not found".to_owned()).into())
                .map(|data| {
                    data.retirement
                        .as_ref()
                        .map(|r| PoolRetirement::from(r.clone()))
                }),
        }
    }
}

struct Status {}

#[juniper::graphql_object(
    Context = Context
)]
impl Status {
    pub fn current_epoch(&self) -> FieldResult<Epoch> {
        // TODO: Would this be the Epoch of last block or a timeframe reference?
        Err(ErrorKind::Unimplemented.into())
    }

    pub async fn latest_block(&self, context: &Context) -> FieldResult<Block> {
        latest_block(context).await.map(Block::from)
    }

    pub async fn epoch_stability_depth(&self, context: &Context) -> String {
        context
            .db
            .blockchain_config
            .epoch_stability_depth
            .to_string()
    }

    pub fn fee_settings(&self, context: &Context) -> FeeSettings {
        let chain_impl_mockchain::fee::LinearFee {
            constant,
            coefficient,
            certificate,
            per_certificate_fees,
            per_vote_certificate_fees,
        } = context.db.blockchain_config.fees;

        FeeSettings {
            constant: Value(format!("{}", constant)),
            coefficient: Value(format!("{}", coefficient)),
            certificate: Value(format!("{}", certificate)),
            certificate_pool_registration: Value(format!(
                "{}",
                per_certificate_fees
                    .certificate_pool_registration
                    .map(|v| v.get())
                    .unwrap_or(certificate)
            )),
            certificate_stake_delegation: Value(format!(
                "{}",
                per_certificate_fees
                    .certificate_stake_delegation
                    .map(|v| v.get())
                    .unwrap_or(certificate)
            )),
            certificate_owner_stake_delegation: Value(format!(
                "{}",
                per_certificate_fees
                    .certificate_owner_stake_delegation
                    .map(|v| v.get())
                    .unwrap_or(certificate)
            )),
            certificate_vote_plan: Value(format!(
                "{}",
                per_vote_certificate_fees
                    .certificate_vote_plan
                    .map(|v| v.get())
                    .unwrap_or(certificate)
            )),
            certificate_vote_cast: Value(format!(
                "{}",
                per_vote_certificate_fees
                    .certificate_vote_cast
                    .map(|v| v.get())
                    .unwrap_or(certificate)
            )),
        }
    }
}

struct Treasury {
    rewards: Value,
    treasury: Value,
    treasury_tax: TaxType,
}

#[juniper::graphql_object(
    Context = Context
)]
impl Treasury {
    fn rewards(&self) -> &Value {
        &self.rewards
    }

    fn treasury(&self) -> &Value {
        &self.treasury
    }

    fn treasury_tax(&self) -> &TaxType {
        &self.treasury_tax
    }
}

#[derive(juniper::GraphQLObject)]
struct FeeSettings {
    constant: Value,
    coefficient: Value,
    certificate: Value,
    certificate_pool_registration: Value,
    certificate_stake_delegation: Value,
    certificate_owner_stake_delegation: Value,
    certificate_vote_plan: Value,
    certificate_vote_cast: Value,
}

#[derive(Clone)]
struct Epoch {
    id: blockcfg::Epoch,
}

impl Epoch {
    fn from_epoch_number(id: EpochNumber) -> FieldResult<Epoch> {
        Ok(Epoch { id: id.try_into()? })
    }

    async fn get_epoch_data(&self, db: &ExplorerDB) -> Option<EpochData> {
        db.get_epoch(self.id).await
    }
}

#[juniper::graphql_object(
    Context = Context
)]
impl Epoch {
    pub fn id(&self) -> EpochNumber {
        self.id.into()
    }

    /// Not yet implemented
    pub fn stake_distribution(&self) -> FieldResult<StakeDistribution> {
        Err(ErrorKind::Unimplemented.into())
    }

    /// Get a paginated view of all the blocks in this epoch
    pub async fn blocks(
        &self,
        first: Option<i32>,
        last: Option<i32>,
        before: Option<IndexCursor>,
        after: Option<IndexCursor>,
        context: &Context,
    ) -> FieldResult<Option<BlockConnection>> {
        let epoch_data = match self.get_epoch_data(&context.db).await {
            Some(epoch_data) => epoch_data,
            None => return Ok(None),
        };

        let epoch_lower_bound = context
            .db
            .get_block(&epoch_data.first_block)
            .await
            .map(|block| u32::from(block.chain_length))
            .expect("Epoch lower bound");

        let epoch_upper_bound = context
            .db
            .get_block(&epoch_data.last_block)
            .await
            .map(|block| u32::from(block.chain_length))
            .expect("Epoch upper bound");

        let boundaries = PaginationInterval::Inclusive(InclusivePaginationInterval {
            lower_bound: 0,
            upper_bound: epoch_upper_bound
                .checked_sub(epoch_lower_bound)
                .expect("pagination upper_bound to be greater or equal than lower_bound"),
        });

        let pagination_arguments = PaginationArguments {
            first,
            last,
            before: before.map(u32::try_from).transpose()?,
            after: after.map(u32::try_from).transpose()?,
        }
        .validate()?;

        BlockConnection::new_async(boundaries, pagination_arguments, |range| async {
            match range {
                PaginationInterval::Empty => unreachable!("No blocks found (not even genesis)"),
                PaginationInterval::Inclusive(range) => context
                    .db
                    .get_main_tip()
                    .await
                    .1
                    .state()
                    .get_block_hash_range(
                        (range.lower_bound + epoch_lower_bound).into(),
                        (range.upper_bound + epoch_lower_bound + 1).into(),
                    )
                    .iter()
                    .map(|(hash, index)| (*hash, u32::from(*index) - epoch_lower_bound))
                    .collect(),
            }
        })
        .await
        .map(Some)
    }

    pub async fn first_block(&self, context: &Context) -> Option<Block> {
        self.get_epoch_data(&context.db)
            .await
            .map(|data| Block::from_valid_hash(data.first_block))
    }

    pub async fn last_block(&self, context: &Context) -> Option<Block> {
        self.get_epoch_data(&context.db)
            .await
            .map(|data| Block::from_valid_hash(data.last_block))
    }

    pub async fn total_blocks(&self, context: &Context) -> BlockCount {
        self.get_epoch_data(&context.db)
            .await
            .map_or(0u32.into(), |data| data.total_blocks.into())
    }
}

struct StakeDistribution {
    pools: Vec<PoolStakeDistribution>,
}

#[juniper::graphql_object(
    Context = Context,
)]
impl StakeDistribution {
    pub fn pools(&self) -> &Vec<PoolStakeDistribution> {
        &self.pools
    }
}

struct PoolStakeDistribution {
    pool: Pool,
    delegated_stake: Value,
}

#[juniper::graphql_object(
    Context = Context,
)]
impl PoolStakeDistribution {
    pub fn pool(&self) -> &Pool {
        &self.pool
    }

    pub fn delegated_stake(&self) -> &Value {
        &self.delegated_stake
    }
}

#[derive(Clone)]
pub struct VotePayloadPublicStatus {
    choice: i32,
}

#[derive(Clone)]
pub struct VotePayloadPrivateStatus {
    proof: ProofOfCorrectVote,
    encrypted_vote: EncryptedVote,
}

#[juniper::graphql_object(
    Context = Context
)]
impl VotePayloadPublicStatus {
    pub fn choice(&self, _context: &Context) -> i32 {
        self.choice
    }
}

#[juniper::graphql_object(
Context = Context
)]
impl VotePayloadPrivateStatus {
    pub fn proof(&self, _context: &Context) -> String {
        let bytes_proof = self.proof.serialize();
        base64::encode_config(bytes_proof, base64::URL_SAFE)
    }

    pub fn encrypted_vote(&self, _context: &Context) -> String {
        let encrypted_bote_bytes = self.encrypted_vote.serialize();
        base64::encode_config(encrypted_bote_bytes, base64::URL_SAFE)
    }
}

#[derive(Clone, GraphQLUnion)]
#[graphql(Context = Context)]
pub enum VotePayloadStatus {
    Public(VotePayloadPublicStatus),
    Private(VotePayloadPrivateStatus),
}

// TODO do proper vote tally
#[derive(Clone)]
pub struct TallyPublicStatus {
    results: Vec<Weight>,
    options: VoteOptionRange,
}

#[juniper::graphql_object(
    Context = Context
)]
impl TallyPublicStatus {
    fn results(&self) -> &[Weight] {
        &self.results
    }

    fn options(&self) -> &VoteOptionRange {
        &self.options
    }
}

#[derive(Clone)]
pub struct TallyPrivateStatus {
    results: Option<Vec<Weight>>,
    options: VoteOptionRange,
}

#[juniper::graphql_object(Context = Context)]
impl TallyPrivateStatus {
    fn results(&self) -> Option<&[Weight]> {
        self.results.as_ref().map(AsRef::as_ref)
    }

    fn options(&self) -> &VoteOptionRange {
        &self.options
    }
}

#[derive(Clone, GraphQLUnion)]
#[graphql(Context = Context)]
pub enum TallyStatus {
    Public(TallyPublicStatus),
    Private(TallyPrivateStatus),
}

#[derive(Clone)]
pub struct VotePlanStatus {
    id: VotePlanId,
    vote_start: BlockDate,
    vote_end: BlockDate,
    committee_end: BlockDate,
    payload_type: PayloadType,
    proposals: Vec<VoteProposalStatus>,
}

impl VotePlanStatus {
    pub async fn vote_plan_from_id(
        vote_plan_id: VotePlanId,
        context: &Context,
    ) -> FieldResult<Self> {
        let vote_plan_id = chain_impl_mockchain::certificate::VotePlanId::from_str(&vote_plan_id.0)
            .map_err(|err| -> juniper::FieldError {
                ErrorKind::InvalidAddress(err.to_string()).into()
            })?;
        if let Some(vote_plan) = context.db.get_vote_plan_by_id(&vote_plan_id).await {
            return Ok(Self::vote_plan_from_data(vote_plan));
        }

        Err(ErrorKind::NotFound(format!(
            "Vote plan with id {} not found",
            vote_plan_id.to_string()
        ))
        .into())
    }

    pub fn vote_plan_from_data(vote_plan: Arc<super::indexing::ExplorerVotePlan>) -> Self {
        let super::indexing::ExplorerVotePlan {
            id,
            vote_start,
            vote_end,
            committee_end,
            payload_type,
            proposals,
        } = (*vote_plan).clone();

        VotePlanStatus {
            id: VotePlanId::from(id),
            vote_start: BlockDate::from(vote_start),
            vote_end: BlockDate::from(vote_end),
            committee_end: BlockDate::from(committee_end),
            payload_type: PayloadType::from(payload_type),
            proposals: proposals
                .into_iter()
                .map(|proposal| VoteProposalStatus {
                    proposal_id: ExternalProposalId::from(proposal.proposal_id),
                    options: VoteOptionRange::from(proposal.options),
                    tally: proposal.tally.map(|tally| match tally {
                        super::indexing::ExplorerVoteTally::Public { results, options } => {
                            TallyStatus::Public(TallyPublicStatus {
                                results: results.into_iter().map(Into::into).collect(),
                                options: options.into(),
                            })
                        }
                        super::indexing::ExplorerVoteTally::Private { results, options } => {
                            TallyStatus::Private(TallyPrivateStatus {
                                results: results
                                    .map(|res| res.into_iter().map(Into::into).collect()),
                                options: options.into(),
                            })
                        }
                    }),
                    votes: proposal
                        .votes
                        .iter()
                        .map(|(key, vote)| match vote.as_ref() {
                            ExplorerVote::Public(choice) => VoteStatus {
                                address: key.into(),
                                payload: VotePayloadStatus::Public(VotePayloadPublicStatus {
                                    choice: choice.as_byte().into(),
                                }),
                            },
                            ExplorerVote::Private {
                                proof,
                                encrypted_vote,
                            } => VoteStatus {
                                address: key.into(),
                                payload: VotePayloadStatus::Private(VotePayloadPrivateStatus {
                                    proof: proof.clone(),
                                    encrypted_vote: encrypted_vote.clone(),
                                }),
                            },
                        })
                        .collect(),
                })
                .collect(),
        }
    }
}

#[juniper::graphql_object(
    Context = Context
)]
impl VotePlanStatus {
    pub fn id(&self) -> &VotePlanId {
        &self.id
    }

    pub fn vote_start(&self) -> &BlockDate {
        &self.vote_start
    }

    pub fn vote_end(&self) -> &BlockDate {
        &self.vote_end
    }

    pub fn committee_end(&self) -> &BlockDate {
        &self.committee_end
    }

    pub fn payload_type(&self) -> &PayloadType {
        &self.payload_type
    }

    pub fn proposals(&self) -> &[VoteProposalStatus] {
        &self.proposals
    }
}

#[derive(Clone)]
pub struct VoteStatus {
    address: Address,
    payload: VotePayloadStatus,
}

#[juniper::graphql_object(
    Context = Context
)]
impl VoteStatus {
    pub fn address(&self) -> &Address {
        &self.address
    }

    pub fn payload(&self) -> &VotePayloadStatus {
        &self.payload
    }
}

#[derive(Clone)]
pub struct VoteProposalStatus {
    proposal_id: ExternalProposalId,
    options: VoteOptionRange,
    tally: Option<TallyStatus>,
    votes: Vec<VoteStatus>,
}

#[juniper::graphql_object(
    Context = Context
)]
impl VoteProposalStatus {
    pub fn proposal_id(&self) -> &ExternalProposalId {
        &self.proposal_id
    }

    pub fn options(&self) -> &VoteOptionRange {
        &self.options
    }

    pub fn tally(&self) -> Option<&TallyStatus> {
        self.tally.as_ref()
    }

    pub fn votes(
        &self,
        first: Option<i32>,
        last: Option<i32>,
        before: Option<IndexCursor>,
        after: Option<IndexCursor>,
    ) -> FieldResult<VoteStatusConnection> {
        let boundaries = if !self.votes.is_empty() {
            PaginationInterval::Inclusive(InclusivePaginationInterval {
                lower_bound: 0u32,
                upper_bound: self
                    .votes
                    .len()
                    .checked_sub(1)
                    .unwrap()
                    .try_into()
                    .expect("tried to paginate more than 2^32 elements"),
            })
        } else {
            PaginationInterval::Empty
        };

        let pagination_arguments = PaginationArguments {
            first,
            last,
            before: before.map(u32::try_from).transpose()?,
            after: after.map(u32::try_from).transpose()?,
        }
        .validate()?;

        VoteStatusConnection::new(boundaries, pagination_arguments, |range| match range {
            PaginationInterval::Empty => vec![],
            PaginationInterval::Inclusive(range) => {
                let from = range.lower_bound;
                let to = range.upper_bound;

                (from..=to)
                    .map(|i: u32| (self.votes[i as usize].clone(), i))
                    .collect::<Vec<(VoteStatus, u32)>>()
            }
        })
    }
}

pub struct Query;

#[juniper::graphql_object(
    Context = Context,
)]
impl Query {
    async fn block(id: String, context: &Context) -> FieldResult<Block> {
        Block::from_string_hash(id, &context.db).await
    }

    async fn block_by_chain_length(
        length: ChainLength,
        context: &Context,
    ) -> FieldResult<Option<Block>> {
        Ok(context
            .db
            .get_main_tip()
            .await
            .1
            .state()
            .find_block_by_chain_length(length.try_into()?)
            .map(Block::from_valid_hash))
    }

    /// query all the blocks in a paginated view
    async fn all_blocks(
        &self,
        first: Option<i32>,
        last: Option<i32>,
        before: Option<IndexCursor>,
        after: Option<IndexCursor>,
        context: &Context,
    ) -> FieldResult<BlockConnection> {
        let longest_chain = latest_block(context).await?.chain_length;

        let block0 = 0u32;

        let boundaries = PaginationInterval::Inclusive(InclusivePaginationInterval {
            lower_bound: block0,
            upper_bound: u32::from(longest_chain),
        });

        let pagination_arguments = PaginationArguments {
            first,
            last,
            before: before.map(u32::try_from).transpose()?,
            after: after.map(u32::try_from).transpose()?,
        }
        .validate()?;

        BlockConnection::new_async(boundaries, pagination_arguments, |range| async {
            match range {
                PaginationInterval::Empty => vec![],
                PaginationInterval::Inclusive(range) => {
                    let a = range.lower_bound.into();
                    let b = range.upper_bound.checked_add(1).unwrap().into();
                    context
                        .db
                        .get_main_tip()
                        .await
                        .1
                        .state()
                        .get_block_hash_range(a, b)
                        .iter_mut()
                        .map(|(hash, chain_length)| (*hash, u32::from(*chain_length)))
                        .collect()
                }
            }
        })
        .await
    }

    async fn transaction(id: String, context: &Context) -> FieldResult<Transaction> {
        let id = FragmentId::from_str(&id)?;

        Transaction::from_id(id, context).await
    }

    fn epoch(id: EpochNumber) -> FieldResult<Epoch> {
        Epoch::from_epoch_number(id)
    }

    fn address(bech32: String) -> FieldResult<Address> {
        Address::from_bech32(&bech32)
    }

    pub async fn stake_pool(id: PoolId, context: &Context) -> FieldResult<Pool> {
        Pool::from_string_id(&id.0, &context.db).await
    }

    pub async fn all_stake_pools(
        &self,
        first: Option<i32>,
        last: Option<i32>,
        before: Option<IndexCursor>,
        after: Option<IndexCursor>,
        context: &Context,
    ) -> FieldResult<PoolConnection> {
        let mut stake_pools = context.db.get_main_tip().await.1.state().get_stake_pools();

        // Although it's probably not a big performance concern
        // There are a few alternatives to not have to sort this
        // - A separate data structure can be used to track InsertionOrder -> PoolId
        // (or any other order)
        // - Find some way to rely in the Hamt iterator order (but I think this is probably not a good idea)
        stake_pools.sort_unstable_by_key(|(id, _data)| id.clone());

        let boundaries = if !stake_pools.is_empty() {
            PaginationInterval::Inclusive(InclusivePaginationInterval {
                lower_bound: 0u32,
                upper_bound: stake_pools
                    .len()
                    .checked_sub(1)
                    .unwrap()
                    .try_into()
                    .expect("tried to paginate more than 2^32 elements"),
            })
        } else {
            PaginationInterval::Empty
        };

        let pagination_arguments = PaginationArguments {
            first,
            last,
            before: before.map(u32::try_from).transpose()?,
            after: after.map(u32::try_from).transpose()?,
        }
        .validate()?;

        PoolConnection::new(boundaries, pagination_arguments, |range| match range {
            PaginationInterval::Empty => vec![],
            PaginationInterval::Inclusive(range) => {
                let from = range.lower_bound;
                let to = range.upper_bound;

                (from..=to)
                    .map(|i: u32| {
                        let (pool_id, stake_pool_data) = &stake_pools[usize::try_from(i).unwrap()];
                        (
                            Pool::new_with_data(
                                certificate::PoolId::clone(pool_id),
                                Arc::clone(stake_pool_data),
                            ),
                            i,
                        )
                    })
                    .collect::<Vec<(Pool, u32)>>()
            }
        })
    }

    pub fn status() -> FieldResult<Status> {
        Ok(Status {})
    }

    pub async fn vote_plan(&self, id: String, context: &Context) -> FieldResult<VotePlanStatus> {
        VotePlanStatus::vote_plan_from_id(VotePlanId(id), context).await
    }

    pub async fn all_vote_plans(
        &self,
        first: Option<i32>,
        last: Option<i32>,
        before: Option<IndexCursor>,
        after: Option<IndexCursor>,
        context: &Context,
    ) -> FieldResult<VotePlanConnection> {
        let mut vote_plans = context.db.get_main_tip().await.1.state().get_vote_plans();

        vote_plans.sort_unstable_by_key(|(id, _data)| id.clone());

        let boundaries = if !vote_plans.is_empty() {
            PaginationInterval::Inclusive(InclusivePaginationInterval {
                lower_bound: 0u32,
                upper_bound: vote_plans
                    .len()
                    .checked_sub(1)
                    .unwrap()
                    .try_into()
                    .expect("tried to paginate more than 2^32 elements"),
            })
        } else {
            PaginationInterval::Empty
        };

        let pagination_arguments = PaginationArguments {
            first,
            last,
            before: before.map(u32::try_from).transpose()?,
            after: after.map(u32::try_from).transpose()?,
        }
        .validate()?;

        VotePlanConnection::new(boundaries, pagination_arguments, |range| match range {
            PaginationInterval::Empty => vec![],
            PaginationInterval::Inclusive(range) => {
                let from = range.lower_bound;
                let to = range.upper_bound;

                (from..=to)
                    .map(|i: u32| {
                        let (_pool_id, vote_plan_data) = &vote_plans[usize::try_from(i).unwrap()];
                        (
                            VotePlanStatus::vote_plan_from_data(Arc::clone(vote_plan_data)),
                            i,
                        )
                    })
                    .collect::<Vec<(VotePlanStatus, u32)>>()
            }
        })
    }
}

pub struct Context {
    pub db: ExplorerDB,
    pub settings: Settings,
}

impl juniper::Context for Context {}

pub type Schema = RootNode<'static, Query, EmptyMutation<Context>, EmptySubscription<Context>>;

pub fn create_schema() -> Schema {
    Schema::new(Query {}, EmptyMutation::new(), EmptySubscription::new())
}

async fn latest_block(context: &Context) -> FieldResult<Arc<ExplorerBlock>> {
    async {
        let hash = context.db.get_main_tip().await.0;
        context.db.get_block(&hash).await
    }
    .await
    .ok_or_else(|| ErrorKind::InternalError("tip is not in explorer".to_owned()))
    .map_err(Into::into)
}
