#![cfg_attr(not(feature = "std"), no_std)]

use ink_lang as ink;

#[ink::contract]
mod request_etl {

    #[ink(event)]
    pub struct Request {
        #[ink(topic)]
        from: AccountId,
        /// PQL ETL Definition
        /// Skip first 2 bytes (hash fn, size) so that we can fit into bytes32
        ipfs_hash: Hash,
    }

    #[ink(storage)]
    pub struct RequestEtl { }

    impl RequestEtl {

        #[ink(constructor)]
        pub fn new() -> Self {
            Self {}
        }

        #[ink(message)]
        pub fn request(&mut self, ipfs_hash: Hash) {
            let from = self.env().caller();
            self.env().emit_event(Request{from, ipfs_hash});
        }

    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use ink_lang as ink;

        extern crate hex;
        use hex::FromHex;

        #[ink::test]
        fn it_works() {
            let mut request_etl = RequestEtl::new();
            // first 2 bytes omitted
            let input = "42978b1c54ad19f93da7dbc05d0f023062256e95360dfba06c09c1605da75a1b";
            let decoded = <[u8; 32]>::from_hex(input).expect("Decoding failed");
            let ipfs_hash = Hash::from(decoded);
            request_etl.request(ipfs_hash);
        }
    }
}
