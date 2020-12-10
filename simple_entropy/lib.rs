#![cfg_attr(not(feature = "std"), no_std)]

use ink_lang as ink;

#[ink::contract]
mod simple_entropy {
    use ink_storage::collections::{HashMap};

    #[derive(Debug, PartialEq, Eq, scale::Encode, scale::Decode)]
    #[cfg_attr(feature = "std", derive(::scale_info::TypeInfo))]
    pub enum Error {
        RequestAlreadyExists,
        PermissionDenied,
    }

    #[ink(event)]
    pub struct Request {
        #[ink(topic)]
        from: AccountId,
        request_id: Hash,
    }

    #[ink(storage)]
    pub struct SimpleEntropy {
        owner: AccountId,
        // HashMap<request_id, result>
        requests: HashMap<Hash, Hash>,
    }

    impl SimpleEntropy {

        #[ink(constructor)]
        pub fn new(owner: AccountId) -> Self {
            Self {
                owner: owner,
                requests: HashMap::new(),
            }
        }

        #[ink(constructor)]
        pub fn default() -> Self {
            Self {
                owner: Self::env().caller(),
                requests: Default::default(),
            }
        }

        #[ink(message)]
        pub fn get_result(&self, request_id: Hash) -> Hash {
            let result = self.requests.get(&request_id).unwrap();
            *result
        }

        #[ink(message)]
        pub fn make_request(&mut self, request_id: Hash) -> Result<(),Error> {
            let caller = self.env().caller();

            if self.requests.contains_key(&request_id) {
                return Err(Error::RequestAlreadyExists);
            } else {
                self.requests.insert(request_id, Hash::from([0x00; 32]));
                self.env().emit_event(Request { from: caller, request_id: request_id});
            }
            Ok(())
        }

        #[ink(message)]
        pub fn write_result(&mut self, request_id: Hash, result: Hash) -> Result<(),Error> {
            let caller = self.env().caller();
            if caller == self.owner {
                self.requests.insert(request_id, result);
            } else {
                return Err(Error::PermissionDenied);
            }
            Ok(())
        }

    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use ink_lang as ink;

        #[ink::test]
        fn it_sets_owner() {
            let owner = AccountId::from([0x0; 32]);
            let c = SimpleEntropy::new(owner);
            assert_eq!(c.owner, owner);
        }

        #[ink::test]
        fn it_makes_new_request() {
            let mut c = SimpleEntropy::default();
            let request_id = Hash::from([0x01; 32]);
            assert_eq!(c.make_request(request_id), Ok(()));
            assert_eq!(c.make_request(request_id), Err(Error::RequestAlreadyExists));
            assert_eq!(c.get_result(request_id), Hash::from([0x00; 32]));
        }

        #[ink::test]
        fn it_accepts_result() {
            let mut c = SimpleEntropy::default();
            let request_id = Hash::from([0x01; 32]);
            let result = Hash::from([0x42; 32]);
            assert_eq!(c.make_request(request_id), Ok(()));
            assert_eq!(c.get_result(request_id), Hash::from([0x00; 32]));
            assert_eq!(c.write_result(request_id, result), Ok(()));
            assert_eq!(c.get_result(request_id), result);
        }

        #[ink::test]
        fn it_rejects_result() {
            // alice is admin
            let accounts = default_accounts();
            set_next_caller(accounts.alice);
            let mut c = SimpleEntropy::new(accounts.alice);
            assert_eq!(c.owner, accounts.alice);

            let request_id = Hash::from([0x01; 32]);
            let result = Hash::from([0x42; 32]);
            assert_eq!(c.make_request(request_id), Ok(()));
            assert_eq!(c.get_result(request_id), Hash::from([0x00; 32]));

            // bob tries to answer
            set_next_caller(accounts.bob);

            assert_eq!(c.write_result(request_id, result), Err(Error::PermissionDenied));
            assert_eq!(c.get_result(request_id), Hash::from([0x00; 32]));
        }


        //
        // helper functions
        //
        const DEFAULT_CALLEE_HASH: [u8; 32] = [0x07; 32];
        const DEFAULT_ENDOWMENT: Balance = 1_000_000;
        const DEFAULT_GAS_LIMIT: Balance = 1_000_000;
        fn default_accounts(
        ) -> ink_env::test::DefaultAccounts<ink_env::DefaultEnvironment> {
            ink_env::test::default_accounts::<ink_env::DefaultEnvironment>()
                .expect("off-chain environment should have been initialized already")
        }

        fn set_next_caller(caller: AccountId) {
            ink_env::test::push_execution_context::<ink_env::DefaultEnvironment>(
                caller,
                AccountId::from(DEFAULT_CALLEE_HASH),
                DEFAULT_ENDOWMENT,
                DEFAULT_GAS_LIMIT,
                ink_env::test::CallData::new(ink_env::call::Selector::new([0x00; 4])),
            )
        }

    }
}
