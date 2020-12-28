#![cfg_attr(not(feature = "std"), no_std)]

use ink_lang as ink;

#[ink::contract]
mod trusted_etl {
    use ink_storage::collections::{HashMap};

    #[derive(Debug, PartialEq, Eq, scale::Encode, scale::Decode)]
    #[cfg_attr(feature = "std", derive(::scale_info::TypeInfo))]
    pub enum Error {
        Unauthorized,
        InternalError,
    }

    #[ink(event)]
    pub struct Request {
        #[ink(topic)]
        from: AccountId,
        /// PQL ETL Definition
        /// Skip first 2 bytes (hash fn, size) so that we can fit into bytes32
        ipfs_hash: Hash,
        /// Block number for request expiry
        valid_till: u64,
    }

    #[ink(storage)]
    pub struct TrustedOracle {
        // TODO: change this to list of authorities
        owner: Option<AccountId>,
        /// Who can deliver the results
        authorized_oracle: AccountId,
        /// Store <RequestId, ExpiryBlock>
        requests: HashMap<u64, u64>,
        /// Current request head
        request_idx: u64,
    }

    impl TrustedOracle {

        #[ink(constructor)]
        pub fn new(owner: Option<AccountId>, oracle: AccountId) -> Self {
            Self {
                owner: owner,
                authorized_oracle: oracle,
                requests: HashMap::new(),
                request_idx: 0,
            }
        }

        #[ink(message)]
        pub fn request(&mut self, ipfs_hash: Hash, valid_period: u32) -> Result<(),Error> {
            let from = self.env().caller();

            if let Some(owner) = self.owner {
                if from != owner {
                    return Err(Error::Unauthorized);
                }
            }
            // loop around to 0 after u64::max_value() is reached
            self.request_idx = self.request_idx.wrapping_add(1);
            // infallible add
            let valid_till = self.env().block_number() + valid_period as u64;
            self.requests.insert(
                self.request_idx,
                valid_till,
            );

            self.env().emit_event(Request{from, ipfs_hash, valid_till});
            Ok(())
        }

        #[ink(message)]
        pub fn callback(&mut self, request_id: u64) -> Result<(),Error> {
            let from = self.env().caller();

            if from != self.authorized_oracle {
                return Err(Error::Unauthorized);
            }

            // TODO: check if request_id has expired

            // TODO: deliver result as callback
            // https://paritytech.github.io/ink/ink_env/fn.invoke_contract.html
            // ???
            // https://paritytech.github.io/ink/ink_env/call/index.html

            // TODO: remove request from storage
            Ok(())
        }

        #[ink(message)]
        pub fn clear_expired(&mut self, request_id: u64) -> Result<(),Error> {
            let from = self.env().caller();

            let owner = match self.owner {
                Some(x) => x,
                None => self.authorized_oracle // hack
            };
            if from != self.authorized_oracle && from != owner {
                return Err(Error::Unauthorized);
            }

            // TODO: check if request_id has expired
            // TODO: remove request from storage
            Ok(())
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
            let mut request_etl = TrustedOracle::new();
            // first 2 bytes omitted
            let input = "42978b1c54ad19f93da7dbc05d0f023062256e95360dfba06c09c1605da75a1b";
            let decoded = <[u8; 32]>::from_hex(input).expect("Decoding failed");
            let ipfs_hash = Hash::from(decoded);
            request_etl.request(ipfs_hash);
        }
    }
}
