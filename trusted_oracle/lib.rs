#![cfg_attr(not(feature = "std"), no_std)]

use ink_lang as ink;

#[ink::contract]
mod trusted_etl {
    use ink_storage::collections::{HashMap};

    #[derive(Debug, PartialEq, Eq, scale::Encode, scale::Decode)]
    #[cfg_attr(feature = "std", derive(::scale_info::TypeInfo))]
    pub enum Error {
        Unauthorized,
        RequestExpired,
        RequestNotExpired,
        RequestNotFound,
        TransferFailed,
        InsufficientFunds,
        BelowSubsistenceThreshold,
        PaymentRequired,
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

    #[ink(event)]
    pub struct RequestInvalidated {
        #[ink(topic)]
        request_id: u64,
        refunded: Balance,
    }

    #[ink(event)]
    pub struct OracleSet {
        #[ink(topic)]
        oracle: AccountId,
    }

    #[ink(event)]
    pub struct UserAdded {
        #[ink(topic)]
        user: AccountId,
    }

    #[ink(event)]
    pub struct UserRemoved {
        #[ink(topic)]
        user: AccountId,
    }

    #[ink(event)]
    pub struct RewardsClaimed {
        #[ink(topic)]
        oracle: AccountId,
        amount: Balance,
    }

    #[ink(event)]
    pub struct FeeChanged {
        #[ink(topic)]
        old_fee: Balance,
        new_fee: Balance,
    }


    #[ink(storage)]
    pub struct TrustedOracle {
        /// Admin of the contract
        owner: AccountId,
        /// Who can make the requests
        authorized_users: HashMap<AccountId, ()>,
        /// Who can deliver the results
        authorized_oracle: AccountId,
        /// Store <RequestId, (AccountId, ExpiryBlock, fee)>
        requests: HashMap<u64, (AccountId, u64, Balance)>,
        /// Current request head
        request_idx: u64,
        /// Current fee per request
        fee: Balance,
    }

    impl TrustedOracle {

        /// Init
        #[ink(constructor)]
        pub fn new(owner: AccountId, oracle: AccountId) -> Self {
            Self {
                owner: owner,
                authorized_users: HashMap::new(),
                authorized_oracle: oracle,
                requests: HashMap::new(),
                request_idx: 0,
                fee: (0 as u128).into(),
            }
        }

        /// In default case the owner is also the user and the oracle
        #[ink(constructor)]
        pub fn default() -> Self {
            let caller = Self::env().caller();
            let mut authorized_users: HashMap<AccountId,()> = HashMap::new();
            authorized_users.insert(caller, ());
            Self {
                owner: caller,
                authorized_oracle: caller,
                authorized_users,
                requests: HashMap::new(),
                request_idx: 0,
                fee: (0 as u128).into(),
            }
        }

        //
        // User Methods
        //

        /// Make a PQL request
        #[ink(message, payable)]
        pub fn request(&mut self, ipfs_hash: Hash, valid_period: u32) -> Result<(),Error> {
            let from = self.env().caller();

            if !self.authorized_users.contains_key(&from) {
                return Err(Error::Unauthorized);
            }

            if self.fee > (0 as u128).into() {
                if self.env().transferred_balance() != self.fee {
                    return Err(Error::PaymentRequired);
                }
            }

            // loop around to 0 after u64::max_value() is reached
            self.request_idx = self.request_idx.wrapping_add(1);
            // infallible add
            let valid_till = self.env().block_number() + valid_period as u64;
            self.requests.insert(
                self.request_idx,
                (from, valid_till, self.fee),
            );

            self.env().emit_event(Request{from, ipfs_hash, valid_till});
            Ok(())
        }

        //
        // Oracle Methods
        //

        /// Deliver the oracle result
        #[ink(message)]
        pub fn callback(&mut self, request_id: u64) -> Result<(),Error> {
            let from = self.env().caller();

            if from != self.authorized_oracle {
                return Err(Error::Unauthorized);
            }

            // check if request_id has expired
            if let Some(request) = self.requests.get(&request_id) {
                let (user_id, valid_till, fee) = request;
                if *valid_till < self.env().block_number() {
                    self.refund_(request_id, *user_id, *fee)?;
                    self.requests.take(&request_id);
                    return Err(Error::RequestExpired);
                }
            } else {
                return Err(Error::RequestNotFound);
            }

            // TODO: deliver result as callback
            // https://paritytech.github.io/ink/ink_env/fn.invoke_contract.html
            // ???
            // https://paritytech.github.io/ink/ink_env/call/index.html

            // remove request from storage
            self.requests.take(&request_id);
            // TODO: emit event
            Ok(())
        }


        /// Distribute the rewards to the oracle.
        #[ink(message)]
        pub fn claim_rewards(&mut self) -> Result<(),Error>{
            let from = self.env().caller();

            if from != self.authorized_oracle {
                return Err(Error::Unauthorized);
            }

            // send rewards to the current oracle
            self.claim_()
        }

        //
        // Admin methods
        //

        /// Distribute the rewards to the oracle.
        #[ink(message)]
        pub fn set_oracle(&mut self, new_oracle: AccountId) -> Result<(),Error>{
            let from = self.env().caller();

            if from != self.owner {
                return Err(Error::Unauthorized);
            }

            // send rewards to the current oracle
            self.claim_()?;

            // set new oracle
            self.authorized_oracle = new_oracle;
            self.env().emit_event(OracleSet{oracle: new_oracle});
            Ok(())
        }

        /// Change the per-request fee.
        #[ink(message)]
        pub fn set_fee(&mut self, new_fee: Balance) -> Result<(),Error>{
            let from = self.env().caller();

            if from != self.owner {
                return Err(Error::Unauthorized);
            }

            let old_fee = self.fee.clone();
            self.fee = new_fee;
            self.env().emit_event(FeeChanged{old_fee, new_fee});
            Ok(())
        }


        /// Add user to the oracle contract
        #[ink(message)]
        pub fn add_user(&mut self, user: AccountId) -> Result<(),Error>{
            let from = self.env().caller();

            if from != self.owner {
                return Err(Error::Unauthorized);
            }

            // add the user
            self.authorized_users.insert(user.clone(), ());
            self.env().emit_event(UserAdded{user});
            Ok(())
        }


        /// Remove user from the oracle contract
        #[ink(message)]
        pub fn remove_user(&mut self, user: AccountId) -> Result<(),Error>{
            let from = self.env().caller();

            if from != self.owner {
                return Err(Error::Unauthorized);
            }

            // remove the user
            self.authorized_users.take(&user);
            self.env().emit_event(UserRemoved{user});
            Ok(())
        }

        /// Remove expired request to free contract storage
        #[ink(message)]
        pub fn clear_expired(&mut self, request_id: u64) -> Result<(),Error> {
            let from = self.env().caller();

            if from != self.authorized_oracle && from != self.owner {
                return Err(Error::Unauthorized);
            }

            if let Some(request) = self.requests.get(&request_id) {
                let (user_id, valid_till, fee) = request;
                if *valid_till < self.env().block_number() {
                    self.refund_(request_id, *user_id, *fee)?;
                    self.requests.take(&request_id);
                    return Ok(());
                } else {
                    return Err(Error::RequestNotExpired);
                }
            } else {
                return Err(Error::RequestNotFound);
            }
        }

        //
        // Other
        //

        // TODO: check if this is private & internal only
        fn claim_(&mut self) -> Result<(),Error> {
            let balance = self.env().balance();
            if balance > (0 as u128).into() {
                let tx = self.env().transfer(self.authorized_oracle, balance);
                return match tx {
                    Ok(_) => {
                        let event = RewardsClaimed{
                            oracle: self.authorized_oracle,
                            amount: balance
                        };
                        self.env().emit_event(event);
                        Ok(())
                    },
                    Err(err) => {
                        match err {
                            ink_env::Error::BelowSubsistenceThreshold =>
                                Err(Error::BelowSubsistenceThreshold),
                            _ => Err(Error::TransferFailed),
                        }
                    }
                }
            }
            Ok(())
        }

        // TODO: check if this is private & internal only
        fn refund_(&mut self, request_id: u64, user_id: AccountId, fee: Balance) -> Result<(),Error> {
            if fee > (0 as u128).into() {
                if self.env().balance() < fee {
                    return Err(Error::InsufficientFunds);
                }
                if let Err(err) = self.env().transfer(user_id, fee) {
                    return match err {
                        ink_env::Error::BelowSubsistenceThreshold =>
                            Err(Error::BelowSubsistenceThreshold),
                        _ => Err(Error::TransferFailed),
                    }
                }
            }
            let event = RequestInvalidated{
                request_id,
                refunded: fee
            };
            self.env().emit_event(event);
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
            let mut request_etl = TrustedOracle::default();
            // first 2 bytes omitted
            let input = "42978b1c54ad19f93da7dbc05d0f023062256e95360dfba06c09c1605da75a1b";
            let decoded = <[u8; 32]>::from_hex(input).expect("Decoding failed");
            let ipfs_hash = Hash::from(decoded);
            request_etl.request(ipfs_hash, 10);
        }
    }
}
