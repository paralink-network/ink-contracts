#![cfg_attr(not(feature = "std"), no_std)]

use ink_lang as ink;

#[ink::contract]
mod simple_entropy {
    use ink_storage::collections::{HashMap};

    #[ink(storage)]
    pub struct SimpleEntropy {
        owner: AccountId,
        requests: HashMap<AccountId, i32>,
    }

    impl SimpleEntropy {

        #[ink(constructor)]
        pub fn new(owner: AccountId) -> Self {
            Self {
                owner: owner,
                requests: HashMap::new()
            }
        }

        #[ink(message)]
        pub fn request(&mut self) {

        }

        #[ink(message)]
        pub fn result(&mut self) {

        }

    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn it_works() {
            let mut simple_entropy = SimpleEntropy::new("");
            assert_eq!(simple_entropy.owner, "");
        }
    }
}
