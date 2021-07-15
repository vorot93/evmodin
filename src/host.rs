use crate::common::{Message, Output};
use anyhow::bail;
use async_trait::async_trait;
use ethereum_types::{Address, H256, U256};

/// State access status (EIP-2929).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AccessStatus {
    Cold,
    Warm,
}

impl Default for AccessStatus {
    fn default() -> Self {
        Self::Cold
    }
}

#[derive(Clone, Copy, Debug)]
pub enum StorageStatus {
    /// The value of a storage item has been left unchanged: 0 -> 0 and X -> X.
    Unchanged,
    /// The value of a storage item has been modified: X -> Y.
    Modified,
    /// A storage item has been modified after being modified before: X -> Y -> Z.
    ModifiedAgain,
    /// A new storage item has been added: 0 -> X.
    Added,
    /// A storage item has been deleted: X -> 0.
    Deleted,
}

/// The transaction and block data for execution.
#[derive(Clone, Debug)]
pub struct TxContext {
    /// The transaction gas price.
    pub tx_gas_price: U256,
    /// The transaction origin account.
    pub tx_origin: Address,
    /// The miner of the block.
    pub block_coinbase: Address,
    /// The block number.
    pub block_number: u64,
    /// The block timestamp.
    pub block_timestamp: u64,
    /// The block gas limit.
    pub block_gas_limit: u64,
    /// The block difficulty.
    pub block_difficulty: U256,
    /// The blockchain's ChainID.
    pub chain_id: U256,
    /// The block base fee per gas (EIP-1559, EIP-3198).
    pub block_base_fee: U256,
}

/// Abstraction that exposes host context to EVM.
///
/// It is asynchronous, allowing for remote access. Errors represent network or host errors.
#[async_trait]
pub trait Host {
    /// Check if an account exists.
    async fn account_exists(&self, address: Address) -> anyhow::Result<bool>;
    /// Get value of a storage key.
    ///
    /// Returns `Ok(H256::zero())` if does not exist.
    async fn get_storage(&self, address: Address, key: H256) -> anyhow::Result<H256>;
    /// Set value of a storage key.
    async fn set_storage(
        &mut self,
        address: Address,
        key: H256,
        value: H256,
    ) -> anyhow::Result<StorageStatus>;
    /// Get balance of an account.
    ///
    /// Returns `Ok(0)` if account does not exist.
    async fn get_balance(&self, address: Address) -> anyhow::Result<U256>;
    /// Get code size of an account.
    ///
    /// Returns `Ok(0)` if account does not exist.
    async fn get_code_size(&self, address: Address) -> anyhow::Result<U256>;
    /// Get code hash of an account.
    ///
    /// Returns `Ok(0)` if account does not exist.
    async fn get_code_hash(&self, address: Address) -> anyhow::Result<H256>;
    /// Copy code of an account.
    ///
    /// Returns `Ok(0)` if offset is invalid.
    async fn copy_code(
        &self,
        address: Address,
        offset: usize,
        buffer: &mut [u8],
    ) -> anyhow::Result<usize>;
    /// Self-destruct account.
    async fn selfdestruct(&mut self, address: Address, beneficiary: Address) -> anyhow::Result<()>;
    /// Call to another account.
    async fn call(&mut self, msg: &Message) -> anyhow::Result<Output>;
    /// Retrieve transaction context.
    async fn get_tx_context(&self) -> anyhow::Result<TxContext>;
    /// Get block hash.
    ///
    /// Returns `Ok(H256::zero())` if block does not exist.
    async fn get_block_hash(&self, block_number: u64) -> anyhow::Result<H256>;
    /// Emit a log.
    async fn emit_log(
        &mut self,
        address: Address,
        data: &[u8],
        topics: &[H256],
    ) -> anyhow::Result<()>;
    /// Mark account as warm, return previous access status.
    ///
    /// Returns `Ok(AccessStatus::Cold)` if account does not exist.
    async fn access_account(&mut self, address: Address) -> anyhow::Result<AccessStatus>;
    /// Mark storage key as warm, return previous access status.
    ///
    /// Returns `Ok(AccessStatus::Cold)` if account does not exist.
    async fn access_storage(&mut self, address: Address, key: H256)
        -> anyhow::Result<AccessStatus>;
}

/// Host that does not support any ops.
pub struct DummyHost;

#[async_trait]
impl Host for DummyHost {
    async fn account_exists(&self, _: Address) -> anyhow::Result<bool> {
        bail!("unsupported")
    }

    async fn get_storage(&self, _: Address, _: H256) -> anyhow::Result<H256> {
        bail!("unsupported")
    }

    async fn set_storage(&mut self, _: Address, _: H256, _: H256) -> anyhow::Result<StorageStatus> {
        bail!("unsupported")
    }

    async fn get_balance(&self, _: Address) -> anyhow::Result<U256> {
        bail!("unsupported")
    }

    async fn get_code_size(&self, _: Address) -> anyhow::Result<U256> {
        bail!("unsupported")
    }

    async fn get_code_hash(&self, _: Address) -> anyhow::Result<H256> {
        bail!("unsupported")
    }

    async fn copy_code(&self, _: Address, _: usize, _: &mut [u8]) -> anyhow::Result<usize> {
        bail!("unsupported")
    }

    async fn selfdestruct(&mut self, _: Address, _: Address) -> anyhow::Result<()> {
        bail!("unsupported")
    }

    async fn call(&mut self, _: &Message) -> anyhow::Result<Output> {
        bail!("unsupported")
    }

    async fn get_tx_context(&self) -> anyhow::Result<TxContext> {
        bail!("unsupported")
    }

    async fn get_block_hash(&self, _: u64) -> anyhow::Result<H256> {
        bail!("unsupported")
    }

    async fn emit_log(&mut self, _: Address, _: &[u8], _: &[H256]) -> anyhow::Result<()> {
        bail!("unsupported")
    }

    async fn access_account(&mut self, _: Address) -> anyhow::Result<AccessStatus> {
        bail!("unsupported")
    }

    async fn access_storage(&mut self, _: Address, _: H256) -> anyhow::Result<AccessStatus> {
        bail!("unsupported")
    }
}
