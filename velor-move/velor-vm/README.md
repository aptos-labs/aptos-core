### VelorVM

VelorVM is used to execute a single Velor transaction.
Executes both system and user transactions.
For every user transaction, the following steps are performed:

1. ***Prologue:***
   Responsible for checking the structure of the transaction such as signature verification, balance checks on the account paying gas for this transaction, etc.
   If prologue checks fail, transaction is discarded and there are no updates to the blockchain state.

2. **Execution:**
   Runs the code specified by the transaction - can be a script or an entry function.
   If transaction payload is a script, it is verified with Move bytecode verifier prior to execution.
   If the payload is an entry function, it is loaded from the code cache (which guarantees that loaded code is verified).
   When the code is verified and loaded, transaction arguments are checked and then the payload is executed via MoveVM that VelorVM wraps. 
   If execution is successful, a change set that can be (but is not yet) applied to the blockchain state is produced.
3. **Aborted execution:**
   If execution is not successful, VelorVM checks if an account needs to be created for this user transaction.
   For more details about sponsored account creation, see [AIP-52](https://github.com/velor-foundation/AIPs/blob/main/aips/aip-52.md).
4. **Epilogue:**
   Post-processing of user transaction, used primarily to charge gas.
   The post-processing runs on top of the blockchain state with the temporary changes made by executing the user code.
   If transaction runs out of gas, the epilogue runs again but on the clean state.
   In the end, the final change set is created, which includes writes created during **Execution**, as well as writes
   created during the epilogue.

### VelorSimulationVM

Used to simulate transactions before executing them on a real network.
Implementation-wise, wraps VelorVM with some minor modifications for multi-signature transactions.

### VelorVMBlockExecutor

Used to execute a block of signature-verified transactions.
Based on the desired concurrency, can run the block sequentially or in parallel (via Block-STM).
