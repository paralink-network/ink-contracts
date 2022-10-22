#![cfg_attr(not(feature = "std"), no_std)]

use ink_lang as ink;

#[ink::contract(env = paralink_ink_contract_extension::ParalinkEnvironment)]
mod paralink_defi_contract {

    use paralink_ink_contract_extension::RoundData;

    #[ink(storage)]
    pub struct ConsumerContract {}

    impl ConsumerContract {
        // Instantiate a new contract
        #[ink(constructor, payable)]
        pub fn new() -> Self {
            Self {}
        }

        /// Return the latest round data received from the SubLink ink! extension
        #[ink(message)]
        pub fn get_latest_round_data(&self, feed_id: u32) -> RoundData {
            self.env()
                .extension()
                .latest_round_data(feed_id)
                .unwrap_or_default()
        }
    }
}
