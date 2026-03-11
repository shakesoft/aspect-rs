//! Database transaction management aspect example.
//!
//! Demonstrates how to automatically wrap database operations in
//! transactions using aspects, ensuring ACID properties without
//! cluttering business logic.

use aspect_core::prelude::*;
use aspect_macros::aspect;
use std::any::Any;
use std::sync::{Arc, Mutex};

/// Simulated database connection
#[derive(Clone)]
struct DbConnection {
    id: usize,
    in_transaction: bool,
}

impl DbConnection {
    fn begin_transaction(&mut self) -> Transaction {
        println!("  [DB] BEGIN TRANSACTION on connection {}", self.id);
        self.in_transaction = true;
        Transaction {
            conn_id: self.id,
            committed: false,
            rolled_back: false,
        }
    }

    fn execute(&self, sql: &str) -> Result<usize, String> {
        if !self.in_transaction {
            return Err("Not in transaction".to_string());
        }
        println!("  [DB] EXEC: {} (conn {})", sql, self.id);
        Ok(1) // Simulated rows affected
    }
}

/// Simulated transaction handle
struct Transaction {
    conn_id: usize,
    committed: bool,
    rolled_back: bool,
}

impl Transaction {
    fn commit(&mut self) -> Result<(), String> {
        if self.rolled_back {
            return Err("Transaction already rolled back".to_string());
        }
        println!("  [DB] COMMIT on connection {}", self.conn_id);
        self.committed = true;
        Ok(())
    }

    fn rollback(&mut self) -> Result<(), String> {
        if self.committed {
            return Err("Transaction already committed".to_string());
        }
        println!("  [DB] ROLLBACK on connection {}", self.conn_id);
        self.rolled_back = true;
        Ok(())
    }
}

impl Drop for Transaction {
    fn drop(&mut self) {
        if !self.committed && !self.rolled_back {
            println!("  [DB] ⚠ Auto-ROLLBACK on drop (conn {})", self.conn_id);
        }
    }
}

/// Thread-local transaction context
static TRANSACTION_CONTEXT: Mutex<Option<Transaction>> = Mutex::new(None);

fn get_transaction() -> Option<Transaction> {
    // Note: This is simplified - in production you'd use thread-locals
    TRANSACTION_CONTEXT.lock().unwrap().take()
}

fn set_transaction(tx: Transaction) {
    *TRANSACTION_CONTEXT.lock().unwrap() = Some(tx);
}

/// Connection pool (simplified)
struct ConnectionPool {
    connections: Vec<Arc<Mutex<DbConnection>>>,
    next_id: usize,
}

impl ConnectionPool {
    fn new() -> Self {
        Self {
            connections: Vec::new(),
            next_id: 0,
        }
    }

    fn get_connection(&mut self) -> Arc<Mutex<DbConnection>> {
        if self.connections.is_empty() {
            let conn = Arc::new(Mutex::new(DbConnection {
                id: self.next_id,
                in_transaction: false,
            }));
            self.next_id += 1;
            self.connections.push(conn.clone());
            conn
        } else {
            self.connections[0].clone()
        }
    }
}

static POOL: Mutex<ConnectionPool> = Mutex::new(ConnectionPool {
    connections: Vec::new(),
    next_id: 0,
});

/// Transaction management aspect
struct TransactionalAspect;

impl Aspect for TransactionalAspect {
    fn around(&self, pjp: ProceedingJoinPoint) -> Result<Box<dyn Any>, AspectError> {
        let function_name = pjp.context().function_name;
        println!("[TX] Starting transaction for {}", function_name);

        // Get connection and start transaction
        let conn = POOL.lock().unwrap().get_connection();
        let mut tx = conn.lock().unwrap().begin_transaction();

        for (i, arg) in pjp.args().iter().enumerate() {
            if let Some(v) = arg.downcast_ref::<u64>() {
                println!("  [TX] Argument {}: type=u64, value={}", i, v);
            } else if let Some(v) = arg.downcast_ref::<f64>() {
                println!("  [TX] Argument {}: type=f64, value={}", i, v);
            } else if let Some(v) = arg.downcast_ref::<String>() {
                println!("  [TX] Argument {}: type=String, value=\"{}\"", i, v);
            } else {
                println!("  [TX] Argument {}: unknown type", i);
            }
        }

        // Execute the function
        match pjp.proceed() {
            Ok(result) => {
                // Success - commit transaction
                match tx.commit() {
                    Ok(_) => {
                        println!("[TX] ✓ Transaction committed for {}", function_name);
                        Ok(result)
                    }
                    Err(e) => {
                        println!("[TX] ✗ Commit failed for {}: {}", function_name, e);
                        let _ = tx.rollback();
                        Err(AspectError::execution(format!("Commit failed: {}", e)))
                    }
                }
            }
            Err(error) => {
                // Error - rollback transaction
                println!(
                    "[TX] ✗ Transaction rolled back for {} due to error",
                    function_name
                );
                let _ = tx.rollback();
                Err(error)
            }
        }
    }
}

// Example transactional operations

#[aspect(TransactionalAspect)]
fn transfer_money(from_account: u64, to_account: u64, amount: f64) -> Result<(), String> {
    println!(
        "  [APP] Transferring ${:.2} from account {} to {}",
        amount, from_account, to_account
    );

    // Get connection
    let conn = POOL.lock().unwrap().get_connection();
    let conn = conn.lock().unwrap();

    // Debit from source account
    conn.execute(&format!(
        "UPDATE accounts SET balance = balance - {} WHERE id = {}",
        amount, from_account
    ))?;

    // Credit to destination account
    conn.execute(&format!(
        "UPDATE accounts SET balance = balance + {} WHERE id = {}",
        amount, to_account
    ))?;

    println!("  [APP] Transfer completed successfully");
    Ok(())
}

#[aspect(TransactionalAspect)]
fn create_user_with_account(username: String, initial_balance: f64) -> Result<u64, String> {
    println!(
        "  [APP] Creating user '{}' with balance ${:.2}",
        username, initial_balance
    );

    let conn = POOL.lock().unwrap().get_connection();
    let conn = conn.lock().unwrap();

    // Insert user
    conn.execute(&format!(
        "INSERT INTO users (username) VALUES ('{}')",
        username
    ))?;
    let user_id = 123; // Simulated generated ID

    // Create account
    conn.execute(&format!(
        "INSERT INTO accounts (user_id, balance) VALUES ({}, {})",
        user_id, initial_balance
    ))?;

    println!("  [APP] User {} created successfully", user_id);
    Ok(user_id)
}

#[aspect(TransactionalAspect)]
fn failing_operation() -> Result<(), String> {
    println!("  [APP] Performing operation that will fail...");

    let conn = POOL.lock().unwrap().get_connection();
    let conn = conn.lock().unwrap();

    // First operation succeeds
    conn.execute("UPDATE users SET last_login = NOW()")?;

    // Second operation fails
    println!("  [APP] Simulating database error...");
    Err("Constraint violation".to_string())
}

fn main() {
    println!("=== Transaction Management Aspect Example ===\n");

    // Example 1: Successful transfer
    println!("1. Successful money transfer:");
    match transfer_money(100, 200, 50.00) {
        Ok(_) => println!("   ✓ Transfer completed\n"),
        Err(e) => println!("   ✗ Transfer failed: {}\n", e),
    }

    // Example 2: Creating user with account
    println!("2. Creating user with account:");
    match create_user_with_account("alice".to_string(), 100.00) {
        Ok(user_id) => println!("   ✓ User created with ID: {}\n", user_id),
        Err(e) => println!("   ✗ Creation failed: {}\n", e),
    }

    // Example 3: Failed operation (automatic rollback)
    println!("3. Operation that fails (automatic rollback):");
    match failing_operation() {
        Ok(_) => println!("   ✗ Unexpected success\n"),
        Err(e) => println!("   ✓ Failed as expected: {} (transaction rolled back)\n", e),
    }

    // Example 4: Multiple operations in sequence
    println!("4. Multiple successful operations:");
    println!("   Transfer 1:");
    let _ = transfer_money(100, 200, 25.00);
    println!("\n   Transfer 2:");
    let _ = transfer_money(200, 300, 15.00);
    println!();

    println!("=== Demo Complete ===");
    println!("\nKey Takeaways:");
    println!("✓ Transactions managed automatically by aspect");
    println!("✓ Business logic clean - no transaction boilerplate");
    println!("✓ Automatic rollback on errors");
    println!("✓ Automatic commit on success");
    println!("✓ ACID properties enforced without code changes");
    println!("\nNote: This uses around() advice to control transaction lifecycle");
}
