use std::collections::VecDeque;
use bitcoin::network::serialize::BitcoinHash;

use blockchain::{BlockChain, BlockDataLike, BlockData};

/// A manager who handles some datas associated with `BlockData`.
/// Internal datas are consecutive.
pub struct BlockAssociatedDataManager<T>
{
    datas: VecDeque<T>,
}

pub trait BlockAssociatedData: BlockDataLike
{
}

impl<T: BlockAssociatedData> BlockAssociatedDataManager<T>
{
    pub fn new() -> BlockAssociatedDataManager<T>
    {
        BlockAssociatedDataManager { datas: VecDeque::new() }
    }

    pub fn len(&self) -> usize
    {
        self.datas.len()
    }

    pub fn minimum_height(&self) -> usize
    {
        self.datas.front().map(|b| b.height()).unwrap_or(0)
    }

    pub fn get_data(&self, block: &BlockData) -> Option<&T>
    {
        let possible_data = self.get_data_by_height(block.height())?;
        if possible_data.bitcoin_hash() == block.bitcoin_hash() {
            Some(possible_data)
        } else {
            None
        }
    }

    pub fn get_data_by_height(&self, height: usize) -> Option<&T>
    {
        let start_height = self.datas.front()?.height();

        if height < start_height {
            return None;
        }

        let idx = height - start_height;
        self.datas.get(idx)
    }

    pub fn contains_data(&self, block: &BlockData) -> bool
    {
        self.get_data(block).is_some()
    }

    /// Returns all blocks which is not contained in `self`.
    /// Note that since blockchain's nature, returned blocks are consecutive.
    pub fn forked_blocks(&self, blockchain: &BlockChain) -> Vec<BlockData>
    {
        let active_chain = blockchain.active_chain();
        let minimum_height = self.minimum_height();
        let associated_blocks = active_chain.iter().skip_while(|b| b.height() < minimum_height);
        let forked_blocks = associated_blocks.skip_while(|b| self.contains_data(b));

        let mut vec = Vec::new();
        for block in forked_blocks {
            vec.push(block.clone());
        }
        vec
    }

    /// Replace data if some data is already stored on same height.
    /// Add data if no data is stored on same height.
    /// Note that `datas` **MUST** be consecutive and **MUST NOT** be empty.
    ///
    /// ```text
    ///                  +---+   +---+   +---+
    /// current data  :  | 0 | - | 1 | - | 2 |
    ///                  +---+   +---+   +---+
    ///
    ///                                  +---+   +---+
    /// new data      :                  | 2'| - | 3'|
    ///                                  +---+   +---+
    /// ------------------------------------------------
    ///
    ///                  +---+   +---+   +---+   +---+
    /// updated data  :  | 0 | - | 1 | - | 2'| - | 3'|
    ///                  +---+   +---+   +---+   +---+
    /// ```
    ///
    /// # Panic
    /// If `datas` is empty
    ///
    /// or
    ///
    /// ```text
    ///                  +---+   +---+
    /// current data  :  | 0 | - | 1 |
    ///                  +---+   +---+
    ///
    ///                                          +---+
    /// new data      :                          | 3'|
    ///                                          +---+
    /// ------------------------------------------------
    ///
    ///     ===== !!!!!!! Panic !!!!!!! =====
    ///
    /// ```
    pub fn update(&mut self, datas: Vec<T>)
    {
        assert!(!datas.is_empty());

        if self.datas.is_empty() {
            self.datas = datas.into();
            return;
        }

        let current_minimum_height = self.minimum_height();
        let current_maximum_height = self.datas.back().unwrap().height();
        let new_minimum_height = datas[0].height();
        assert!(new_minimum_height <= current_maximum_height + 1);
        self.datas.truncate(new_minimum_height - current_minimum_height);

        self.datas.append(&mut datas.into());
    }

    pub fn pop(&mut self) -> Option<T>
    {
        self.datas.pop_front()
    }
}
