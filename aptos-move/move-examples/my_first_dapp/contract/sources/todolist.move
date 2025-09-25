module todolist_addr::todolist {
    use aptos_framework::event;
    use aptos_std::table::{Self, Table};
    use std::signer;
    use std::string::String; 

    #[test_only]
    use aptos_framework::account;
    #[test_only]
    use std::string::{Self}; 

    // Errors
    /// Account has not initialized a todo list
    const ENOT_INITIALIZED: u64 = 1;
    /// Task does not exist
    const ETASK_DOESNT_EXIST: u64 = 2;
    /// Task is already completed
    const ETASK_IS_COMPLETED: u64 = 3;

    #[event]
    struct TaskCreated has drop, store {
        task_id: u64,
        creator_addr: address,
        content: String,
        completed: bool,
    }

    /// Main resource that stores all tasks for an account
    struct TodoList has key {
        tasks: Table<u64, Task>,
        task_counter: u64
    }

    /// Individual task structure
    struct Task has store, drop, copy {
        task_id: u64,
        creator_addr: address,
        content: String,
        completed: bool,
    }

    /// Initializes a new todo list for the account
    public entry fun create_list(account: &signer) {
        let tasks_holder = TodoList {
            tasks: table::new(),
            task_counter: 0
        };
        // Move the TodoList resource under the signer account
        move_to(account, tasks_holder);
    }

    /// Creates a new task in the todo list
    public entry fun create_task(account: &signer, content: String) acquires TodoList {
        // Get the signer address
        let signer_address = signer::address_of(account);

        // Ensure the account has initialized a todo list
        assert!(exists<TodoList>(signer_address), ENOT_INITIALIZED);

        // Get the TodoList resource
        let todo_list = borrow_global_mut<TodoList>(signer_address);
        
        // Increment task counter
        let counter = todo_list.task_counter + 1;
        
        // Create a new task
        let new_task = Task {
            task_id: counter,
            creator_addr: signer_address,
            content,
            completed: false
        };
        
        // Add the new task to the tasks table
        todo_list.tasks.upsert(counter, new_task);
        
        // Update the task counter
        todo_list.task_counter = counter;
        
        // Emit a task created event
        event::emit(TaskCreated {
            task_id: counter,
            creator_addr: signer_address,
            content,
            completed: false
        })
    }

    /// Marks a task as completed
    public entry fun complete_task(account: &signer, task_id: u64) acquires TodoList {
        // Get the signer address
        let signer_address = signer::address_of(account);
        
        // Ensure the account has initialized a todo list
        assert!(exists<TodoList>(signer_address), ENOT_INITIALIZED);
        
        // Get the TodoList resource
        let todo_list = borrow_global_mut<TodoList>(signer_address);
        
        // Ensure the task exists
        assert!(todo_list.tasks.contains(task_id), ETASK_DOESNT_EXIST);
        
        // Get the task record
        let task_record = todo_list.tasks.borrow_mut(task_id);
        
        // Ensure the task is not already completed
        assert!(task_record.completed == false, ETASK_IS_COMPLETED);
        
        // Mark the task as completed
        task_record.completed = true;
    }

    #[test(admin = @0x123)]
    public entry fun test_flow(admin: signer) acquires TodoList {
        // Create an admin account for testing
        account::create_account_for_test(signer::address_of(&admin));
        
        // Initialize a todo list for the admin account
        create_list(&admin);

        // Create a task and verify it was added correctly
        create_task(&admin, string::utf8(b"Create e2e guide video for aptos devs."));
        let todo_list = borrow_global<TodoList>(signer::address_of(&admin));
        assert!(todo_list.task_counter == 1, 5);
        
        // Verify task details
        let task_record = todo_list.tasks.borrow(todo_list.task_counter);
        assert!(task_record.task_id == 1, 6);
        assert!(task_record.completed == false, 7);
        assert!(task_record.content == string::utf8(b"Create e2e guide video for aptos devs."), 8);
        assert!(task_record.creator_addr == signer::address_of(&admin), 9);

        // Complete the task and verify it was marked as completed
        complete_task(&admin, 1);
        let todo_list = borrow_global<TodoList>(signer::address_of(&admin));
        let task_record = todo_list.tasks.borrow(1);
        assert!(task_record.task_id == 1, 10);
        assert!(task_record.completed == true, 11);
        assert!(task_record.content == string::utf8(b"Create e2e guide video for aptos devs."), 12);
        assert!(task_record.creator_addr == signer::address_of(&admin), 13);
    }

    #[test(admin = @0x123)]
    #[expected_failure(abort_code = ENOT_INITIALIZED)]
    public entry fun account_can_not_update_task(admin: signer) acquires TodoList {
        // Create an admin account for testing
        account::create_account_for_test(signer::address_of(&admin));
        
        // Attempt to complete a task without creating a list first (should fail)
        complete_task(&admin, 2);
    }
}