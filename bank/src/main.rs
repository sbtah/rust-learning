#[derive(Debug)]
struct Account {
    id: u32,
    ballance: i32,
    holder: String,
}


#[derive(Debug)]
struct Bank {
    accounts: Vec<Account>,
}

impl Bank {
    fn new() -> Self {
        return Bank{ accounts: Vec::new() };
    }

    fn add_account(&mut self, account: Account) {
        self.accounts.push(account);
    }

    fn get_last_id(&self) -> u32 {
        match self.accounts.last() {
            Some(acc) => acc.id + 1,
            None => 1,
        }
    }

    fn total_ballance(&self) -> i32 {

        // let mut total = 0;

        // for account in &self.accounts {
        //     total += account.ballance;
        // }

        // return total;
        self.accounts.iter().map(|account| account.ballance).sum()
    }

    fn summary(&self) -> Vec<String> {
        self.accounts
            .iter()
            .map(|account|account.summary())
            .collect()
    }
}


impl Account {
    fn new(id: u32, holder: String) -> Self {
        return Account {
            id: id,
            ballance: 0,
            holder: holder
        };
    }

    fn deposit(&mut self, amount: i32) -> &i32 {
        self.ballance += amount;
        &self.ballance
    }

    fn withdraw(&mut self, amount: i32) -> &i32 {
        self.ballance -= amount;
        &self.ballance
    }

    fn summary(&self) -> String {
        let summary: String = format!(
            "Account ID: {} Holder: {}, Ballance: {}", self.id, self.holder, self.ballance
        );
        return summary;
    }
}

// &Account - means we want this function argument to be a reference to a value.


fn main() {
    let mut bank: Bank = Bank::new();

    let first_id: u32 = bank.get_last_id();
    let mut account: Account = Account::new(first_id, String::from("Grzegorz"));
    account.deposit(30);
    account.withdraw(10);
    println!("{}", account.summary());
    bank.add_account(account);


    let second_id: u32 = bank.get_last_id();
    let mut second_account: Account = Account::new(second_id, String::from("Momo"));
    second_account.deposit(90);
    bank.add_account(second_account);

    println!("TOTAL: {:#?}", bank.total_ballance());
    println!("SUMMARY: {:#?}", bank.summary());

    println!("{:#?}", bank)
}
