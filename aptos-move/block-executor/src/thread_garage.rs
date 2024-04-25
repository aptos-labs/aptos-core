use crate::thread_garage_barrier::ThreadGarageBarrier;
use crate::explicit_sync_wrapper::ExplicitSyncWrapper;


use std::fmt::{Debug, Display};
use std::sync::MutexGuard;
use std::sync::{
    atomic::{AtomicUsize, Ordering, AtomicBool},
    Mutex,
    Condvar,
    Arc,
};

use crossbeam::utils::CachePadded;

use std::any::Any;
use std::thread;
use std::result::Result;

pub enum SuspendResult<T>
{
        FailedDueToHaltedGarage,
        WokenUpToHaltedGarage,
 //       ErrorNoAvailableThreads,
        FailedRegisteringHook,
        NotHalted(T),
}


//struct used for waking up thread with index thread_id, 
//value can used to pass to awoken thread
#[derive(Debug)]
pub struct Baton<T> 
{

     thread_id: usize,
     value: Arc<ExplicitSyncWrapper<T>>,   
}

impl <T> Baton<T> where T: PartialEq + Copy + Clone {
    pub fn new(thread_id: usize, value: T) -> Self {
        Self {
            thread_id,
            value: Arc::new(ExplicitSyncWrapper::new(value)),
        }
    }

    pub fn change_value(self, new_value: T) -> Self {
        let write_ref = self.value.dereference_mut();
        *write_ref = new_value;
        
        self.value.unlock();

        self
    } 

    pub fn get_value(&self) -> T {
        let _lock = self.value.acquire();
        return *(self.value.dereference());
    }

    pub fn get_thread_id(&self) -> usize {
        self.thread_id
    }
}


impl<T> Clone for Baton<T> where T: PartialEq + Copy +  Clone {
    fn clone(&self) -> Self {
        Self {
            thread_id: self.thread_id,
            value: self.value.clone(),
        }
    }
}

// return type of worker function, empty means that it successfully finished running worker function, 
// otherwise it contains index of thread to wake up
pub struct ReturnType { 
    baton_thread_id: Option<usize>,
    baton_wrapper: Option<Box<dyn Any>>,
}

impl ReturnType {
    //get baton and initialize returntype with the index of thread to be woken up (currently, value is not used)
    pub fn new<T: PartialEq+Copy+Clone+'static> (baton: Option<Baton<T>>) -> Self { 
        match baton {
            Some(baton) => {
                Self {
                    baton_thread_id: Some(baton.thread_id),
                    baton_wrapper: Some(Box::new(baton)),
                }
            }
            None => {
                Self {
                    baton_thread_id: None,
                    baton_wrapper: None,
                }
            }
        }
    } 

    //try to retrieve baton, consumes self
    //Return None if baton did not exist
    //Retrun Some(Ok(Baton<T>)) if baton exits, and downcast successfull
    //otherwise return copy of Box<dyn Any> containing Baton, maybe can be used to retry downcast
    pub fn get_baton<T: PartialEq+Copy+Display+'static>(self) -> Option<Result<Baton<T>, Box<dyn Any>>> {
        match self.baton_wrapper {
            Some(baton_wrapper) => {
                match baton_wrapper.downcast::<Baton<T>>() {
                    Ok(baton_box) => Some(Ok(*baton_box)),
                    Err(baton_wrapper) => Some(Err(baton_wrapper)),
                }
            },
            None => None,
        }
    } 
}


pub struct ThreadGarageHandle<'a>{ //Handle to the ThreadGarage given to the worker function as input
    thread_id: usize,
    garage: &'a Arc<ThreadGarage>,
}


impl<'a> ThreadGarageHandle<'a>{
    pub fn new(garage: &'a Arc<ThreadGarage>, thread_id: usize) -> Self {
        Self {
            garage,
            thread_id
        }
    }

    pub fn get_thread_id(&self) -> usize {
        self.thread_id
    }

    //suspend current thread; creates Baton with current thread_id and passes it as argument to register_hook 
    
    pub fn conditional_suspend<T,F,E>  (&self, register_hook: F, default_value: T)-> Result<SuspendResult<T>,E>
        where T : PartialEq + Copy + Debug, F : Fn(Baton<T>) -> Option<Result<T,E>> {
        return self.garage.conditional_suspend(register_hook, default_value, self.thread_id);
    }

    pub fn halt(&self) -> bool {
        self.garage.halt()
    }

    pub fn is_halted(&self) -> bool {
        self.garage.is_halted()
    }
}


pub struct CurrentThreadData {
    num_active: usize, //number of threads currently running worker function
    num_completed: usize, //number of threads which finished running worker function
    num_sleeping: usize,
    halting_cleanup_idx: usize,
    asleep: Vec<bool> //asleep[i] is true if thread i is asleep, waiting for dependencies to be satisfied
}

impl CurrentThreadData {
    pub fn new(capacity: usize) -> Self {
        Self {
            num_active: 0,
            num_completed: 0,
            num_sleeping: 0,
            halting_cleanup_idx: 0,
            asleep: vec![false; capacity],
        }
    }
}

pub trait Executor {
    unsafe fn execute(input: *const (), garage: &Arc<ThreadGarage>, thread_id: usize);
}

struct WorkerFunctionWrapper {
    input: *const (),
    executor: unsafe fn(*const (),  &Arc<ThreadGarage>, usize),
}

impl WorkerFunctionWrapper {
    pub fn new<F> (input: *const F) -> Self where F : Executor {  
        Self {
            input: input as *const(),
            executor: <F as Executor>::execute,
        }
    }    

    //IMPORTANT needs to be executed exactly once, even though it takes &self
    pub fn execute(&self, garage: &Arc<ThreadGarage>, thread_id: usize) {
        unsafe {
           (self.executor)(self.input, garage, thread_id);
        }
    }
}

unsafe impl Send for WorkerFunctionWrapper {}
unsafe impl Sync for WorkerFunctionWrapper {}

pub struct ThreadGarage {
    // Max number of threads allowed to run the worker function.
    max_active: usize,
    
    // Limit of how many threads we are allowed to spawn.
    max_spawned: usize, // >= max_active

    // number of currently spawned threads
    num_spawned: AtomicUsize,

    //mutex to the vector containing worker functions
    //main thread pushes to it, worker threads remove from it

    worker_function_vec: ExplicitSyncWrapper<Vec<Option<WorkerFunctionWrapper>>>,
    
    //global_barrier used for synchronizing threads, initialized with max_spawned
    global_barrier: ThreadGarageBarrier,

    //mutex protecting mutable shared data
    global_mutex: CachePadded<Mutex<CurrentThreadData>>,
    
    //global condvar used for waking up threads which are asleep due to enough threads running worker function
    global_cv: Condvar,

    //vector of condvars used to waking up threads which are asleep due to unsatisfied dependencies
    baton_cv: Vec<Condvar>,
    
    halted: CachePadded<AtomicBool>,
}

impl ThreadGarage {
    fn new(max_active: usize, max_spawned: usize) -> Self {
        Self {
            max_active,
            max_spawned,
            num_spawned: AtomicUsize::new(0),
            worker_function_vec: ExplicitSyncWrapper::new((0..max_spawned).map(|_| None).collect()),
            global_barrier: ThreadGarageBarrier::new(max_spawned),
            global_mutex: CachePadded::new(Mutex::new(CurrentThreadData::new(max_spawned))),
            global_cv: Condvar::new(),
            baton_cv: (0..max_spawned).map(|_| Condvar::new()).collect(),
            halted: CachePadded::new(AtomicBool::new(false)),
        }
    }

    pub fn conditional_suspend<T,F,E> (&self, register_hook:F, default_value: T, thread_id: usize) -> Result<SuspendResult<T>, E>
        where T : PartialEq + Copy + Debug, F: Fn(Baton<T>)->Option<Result<T,E>> {
        //println!("{thread_id}");
        
        //eprintln!("suspend called on thread={}", thread_id);
        //create new baton for current_thread, using default_value
        let baton: Baton<T>;
        {
            let mut lock = self.global_mutex.lock().unwrap();
            
            if self.is_halted() {
                return Ok(SuspendResult::FailedRegisteringHook);
            }

            lock.asleep[thread_id] = true;
            baton = Baton::new(thread_id, default_value);
        }
        
        //pass baton clone to register hook, so that it can be processed and used for waking the current thread up later
        let hook_result = register_hook(baton.clone());
        

        //eprintln!("returned from hook, thread = {}", thread_id);
        match hook_result {
            Some(val) => {
                eprintln!("dependency already resolved thread={}", thread_id);
                //depencency already satisfied, no need to suspend
                {
                    let mut lock = self.global_mutex.lock().unwrap();
                    if !lock.asleep[thread_id] {
                        lock.num_active -= 1;
                        self.global_cv.notify_one();
                    }
                    else {
                        lock.asleep[thread_id] = false;
                    }
                }
                if self.is_halted() {
                    //eprintln!("suspend not possible due to halt thread={}", thread_id);
                    return Ok(SuspendResult::FailedDueToHaltedGarage);
                }

                match val {
                    Ok(val) => {
                        return Ok(SuspendResult::NotHalted(val));
                    },
                    Err(e) => {
                        //eprintln!("returning error thread={}", thread_id);
                        return Err(e);
                    },
                };
            },
            None => {
                //eprintln!("actually going to sleep thread={}", thread_id);
                //suspend logic
                let mut lock = self.global_mutex.lock().unwrap();

                /*if lock.num_sleeping + 1 == self.max_spawned {
                    if !lock.asleep[thread_id] {
                        lock.num_active -= 1;
                        //self.global_cv.notify_one();
                    }
                    else {
                        lock.asleep[thread_id] = false;
                    }
                    return Err(SuspendError::ErrorNoAvailableThreads);
                }*/

                lock.num_active -= 1;
                lock.num_sleeping += 1;

                //notify one of the threads that since current thread is suspended, it can proceed with running main function

                self.global_cv.notify_one();

                //eprintln!("num sleeping prior {}", lock.num_sleeping);

                while lock.asleep[thread_id] {
                    lock = self.baton_cv[thread_id].wait(lock).unwrap();
                }

                //eprintln!("woke up thread={}", thread_id);
                lock.num_sleeping -= 1;

                //eprintln!("num sleeping after {}", lock.num_sleeping);

                if self.is_halted() {
                    //println!("Thread: {} woke up in halted state", thread_id);
                    return Ok(SuspendResult::WokenUpToHaltedGarage);
                }

                let val = baton.get_value();

                //thread which woke up current thread, should have changed default value
                assert!(val != default_value, "failed with {:?}=={:?}", val, default_value);
                return Ok(SuspendResult::NotHalted(val));
            },
        };
    }

    //main thread loop
    fn thread_loop(self: Arc<Self>, thread_id: usize) {
        // on construction
        {
            self.num_spawned.fetch_add(1, Ordering::Relaxed);
            self.global_barrier.arrive_and_wait(); // wait for all worker threads to arrive at this point
        }

        // run the worker function
        loop {
            // Wait for the next worker_function or for the signal to
            // destroy the garage
            self.global_barrier.arrive_and_wait();
             //main threads sets worker function and then calls arrive_and_wait, hence we are ready to proceed
            
            let temp = self.worker_function_vec.dereference();
            match &temp[thread_id] {
                Some(fn_to_run) => fn_to_run.execute(&self, thread_id),
                None => { 
                    break
                },
            };            

            // Logic to actually run fn_to_run until N successes are
            // done
            self.global_barrier.arrive_and_wait();
        }
        
        //ready to drop, no need to wait for other threads
        {
            self.num_spawned.fetch_sub(1, Ordering::Relaxed);
            self.global_barrier.arrive_and_drop();
        }
    }
    

    pub fn halt(&self) -> bool {
        !self.halted.swap(true, Ordering::SeqCst)
    }

    pub fn is_halted(&self) -> bool {
        self.halted.load(Ordering::SeqCst)
    }

    pub fn halting_cleanup_pass(self: &Arc<Self>, lock: &mut MutexGuard<CurrentThreadData>, thread_id: usize) -> bool {
        if self.is_halted() {
            while lock.halting_cleanup_idx < self.max_spawned {
                let halting_cleanup_idx = lock.halting_cleanup_idx;
                //println!("IDx: << {} <<  cleanup_idx: {}", thread_id, halting_cleanup_idx);
                //println!(" num_active: {} num_completed: {}", lock.num_active, lock.num_completed);
                if lock.asleep[lock.halting_cleanup_idx] {
                    //println!("WILL WAKEUP THREAD: {}", lock.halting_cleanup_idx);
                    lock.asleep[halting_cleanup_idx] = false;
                    self.baton_cv[halting_cleanup_idx].notify_one();
                    lock.halting_cleanup_idx += 1;
                    return true;
                }
                lock.halting_cleanup_idx += 1;
            }
        }
        //println!("Clean exit from thread: {}",  thread_id); 
        false
    }

    pub fn can_complete(self: &Arc<Self>, thread_id: usize) -> bool {    
        let mut lock = self.global_mutex.lock().unwrap();

        if !self.halting_cleanup_pass(&mut lock, thread_id) {
            lock.num_active -= 1;
            lock.num_completed += 1;

            if lock.num_completed == self.max_active {
                self.global_cv.notify_all();
            }
            return true;
        }
        false
    }
}


struct WorkerFunctionExecutor<F> {
    worker_function: F,
}

impl<F> WorkerFunctionExecutor<F> where F : Fn(&ThreadGarageHandle) -> ReturnType + Send + Sync {
    pub fn new(worker_function: F) -> Self {
        Self {
            worker_function
        }
    }

    pub(super) fn get_worker_function_wrapper(self: &Arc<Self>) -> WorkerFunctionWrapper {
        let temp = Arc::into_raw(Arc::clone(self));
        WorkerFunctionWrapper::new(temp)
    }
}

impl<F> Executor for WorkerFunctionExecutor<F> where F : Fn(&ThreadGarageHandle) -> ReturnType + Send + Sync {
    unsafe fn execute (input: *const (), garage: &Arc<ThreadGarage>, thread_id: usize) {
        let input = Arc::from_raw(input as *mut Self);
        let handle = ThreadGarageHandle::new(garage, thread_id);
        loop {
            // we need to check what to do from:
            // 1. run worker function
            // 2. wait for someone to call suspend
            // 3. return (we're done)
            {
                let mut lock = garage.global_mutex.lock().unwrap();

                assert!(lock.num_active + lock.num_completed <= garage.max_active);

                while lock.num_active + lock.num_completed == garage.max_active { 

                    // Need to check before waiting; in case of equality, we have completed worker function max number of times
                    if lock.num_completed == garage.max_active {
                        return;
                    }
                    
                     // Notified from:
                    // 1. the last completed worker
                    // 2. a suspended worker
                    lock = garage.global_cv.wait(lock).unwrap();

                    assert!(lock.num_active + lock.num_completed <= garage.max_active);
                }
                // we know we'll be running the worker function
                lock.num_active += 1;
            }
            
            if garage.is_halted() {
                if garage.can_complete(thread_id) {
                    return;   
                }          
                continue;
            }

            //println!("Calling closure, thread: {}", thread_id);
            let return_value = (input.worker_function)(&handle);

            let baton_thread_id = return_value.baton_thread_id;

            match baton_thread_id {
                Some(baton_thread_id) if !garage.is_halted() => {
                    let mut lock = garage.global_mutex.lock().unwrap();
                    
                    if lock.asleep[baton_thread_id] {
                        lock.asleep[baton_thread_id] = false;
                        //println!("thread: {}, waking up thread: {}", thread_id, baton_thread_id);
                        garage.baton_cv[baton_thread_id].notify_one();
                    }
                    else {
                        lock.num_active -= 1;
                    }
                },
                _ => {
                    if garage.can_complete(thread_id) {
                        return;   
                    } 
                }
            };
        }
        //(input.worker_function)(handle)
    }
}

pub struct ThreadGarageExecutor {
    garage: Arc<ThreadGarage>,
    max_spawned: usize,
}


impl ThreadGarageExecutor {    
    

    pub fn num_total_threads(&self) -> usize {
        self.max_spawned
    }

    pub fn new(max_active: usize, max_spawned: usize) -> Self {
        assert!(max_spawned >= max_active);
        assert!(max_active > 0usize);

        let temp_thread_garage = Arc::new(ThreadGarage::new(max_active, max_spawned));
        
        for idx in 1..temp_thread_garage.max_spawned {
            let c = temp_thread_garage.clone(); 
            let _worker = thread::spawn(move || { 
                c.thread_loop(idx);
            });
        }

        temp_thread_garage.global_barrier.arrive_and_wait(); //signal worker functions to proceed to loop

        Self {
            garage: temp_thread_garage,
            max_spawned,
        }
    }


    pub fn spawn_n<F> (&self, worker_function: F) where F : Fn(&ThreadGarageHandle) -> ReturnType  + Send + Sync {        
        //start threads, first time spawn_n is called

        let temp_executor = Arc::new(WorkerFunctionExecutor::new(worker_function));
        {

            let mut lock = self.garage.global_mutex.lock().unwrap();
            lock.num_completed = 0;
            lock.halting_cleanup_idx = 0;
        }
        self.garage.halted.store(false, Ordering::Release);
        
        //worker threads are waiting on barrier, hence it is safe to modify worker function vector 
        {
            let temp_worker_function_vec = self.garage.worker_function_vec.dereference_mut();
            for idx in 1..self.garage.max_spawned {
                temp_worker_function_vec[idx] = Some(temp_executor.get_worker_function_wrapper());
            }
        }  
            
        self.garage.global_barrier.arrive_and_wait(); //signal worker threads that they can call handle_fn_to_run
        
        //main thread also calls handle_fn_to_run
        
        let worker = temp_executor.get_worker_function_wrapper();
        worker.execute(&self.garage, 0); 

        self.garage.global_barrier.arrive_and_wait();
        
        //all threads are finished with current worker function

    }
    
}



impl Drop for ThreadGarageExecutor{
    fn drop(&mut self) {

        //signal worker threads to finish
        {
            let temp_worker_function_vec = self.garage.worker_function_vec.dereference_mut();
            for idx in 1..self.garage.max_spawned {
                temp_worker_function_vec[idx] = None;
            }
        }
        
        //signal worker functions to read current worker function, which will be None, hence they will break the loop
        self.garage.global_barrier.arrive_and_wait();
        
        // Workers know that they should quit
        // and start dropping on the barrier
        self.garage.global_barrier.arrive_and_wait();
    }
}
   



