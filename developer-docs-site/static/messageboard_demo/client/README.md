# MessageBoard

### Setup and Run the Demo

Download Aptos Core

```bash
git clone https://github.com/aptos-labs/aptos-core.git
cd aptos-core
./scripts/dev_setup.sh
source ~/.cargo/env
```

Install Apto Cli [doc](https://github.com/aptos-labs/aptos-core/tree/main/crates/aptos)

```bash
cargo install --git https://github.com/aptos-labs/aptos-core.git aptos
```

Run demo using the commands below: 

```bash
cd developer-docs-site/static/messageboard_demo/client
yarn install
yarn demo
```

**This demo shows:**

1. Write a basic smart contract with move [source](https://github.com/aptos-labs/aptos-core/tree/main/aptos-move/move-examples/messageboard)
2. Publish, create and interact with messageboard [source](https://github.com/aptos-labs/aptos-core/tree/main/developer-docs-site/static/messageboard_demo/client).

A messageboard has an admin who is the creator of the messageboard.

The messageboard has a `pinned_post` resource. Only participants who have the authority can modify this `pinned_post` resource. Admin can provide this authority to the participants.

Besides the `pinned_post` resource, anyone can send a message to the messageboard. This will trigger a `MessageChangeEvent`. Anyone can subscribe to the event stream to read all the messages sent.

## Messageboard Smart Contract

There are two types of messageboards: capability-based messageboard and ACL-based messageboard. The capability-based messageboard is generally more secure (e.g.,  [confused deputy problem](https://en.wikipedia.org/wiki/Confused_deputy_problem) ) and also prevents the problem of doing ACL validation against a long list.

The source code can be found here [source](https://github.com/aptos-labs/aptos-core/tree/main/aptos-move/move-examples/messageboard).

### Capability-based Messageboard

This [messageboard](https://github.com/aptos-labs/aptos-core/blob/main/aptos-move/move-examples/messageboard/sources/CAPMessageBoard.move) uses MessageChangeCapability to control who can change the pinned_post.

```tsx
struct MessageChangeCapability has key, store {
    board: address
}
```

When a participant wants to modify the pinned post, the participant first acquires the capability by borrowing the message change capability resource (`MessageChangeCapability`). Next, a check is made if the cap's board is the same as the board address for which the participant is trying to change the `pinned_post`.
```tsx
public(script) fun send_pinned_message(
        account: &signer, board_addr: address, message: vector<u8>
    ) acquires MessageChangeCapability, MessageChangeEventHandle, CapBasedMB {
        let cap = borrow_global<MessageChangeCapability>(Signer::address_of(account));
        assert!(cap.board == board_addr, EACCOUNT_NO_NOTICE_CAP);
        let board = borrow_global_mut<CapBasedMB>(board_addr);
        board.pinned_post = message;
       ...
    }
```

To provide participant the capability resource, the board creator creates the `MessageChangeCapability` and offer the capability to a participant’s address. 

```tsx
public(script) fun add_participant(account: &signer, participant: address) acquires MessageCapEventHandle {
    let board_addr = Signer::address_of(account);
    Offer::create(account, MessageChangeCapability{ board: board_addr }, participant);
...
}
```

The participant can then claim this capability and move it to their own account. Later on, the participant can use this resource stored in their account to modify the `pinned_post`.
```tsx
let notice_cap = Offer::redeem<MessageChangeCapability>(
            account, board);
move_to(account, notice_cap);
```

Move also supports emitting events. These events will be stored in event stream that can be queried through API request. For example, any one can send a message to the messageboard if they don’t want to modify the pinned post. The contract will emit an event and add the event to the board’s event stream. Thus anyone subscribed to the event stream of the board can see all the messages.
```tsx
public(script) fun send_message_to(
    account: signer, board_addr: address, message: vector<u8>
) acquires MessageChangeEventHandle {
    let event_handle = borrow_global_mut<MessageChangeEventHandle>(board_addr);
    Event::emit_event<MessageChangeEvent>(
        &mut event_handle.change_events,
        MessageChangeEvent{
            message,
            participant: Signer::address_of(&account)
        }
    );
}
```

### ACL-Based Messageboard

[ACL-Based messageboard](https://github.com/aptos-labs/aptos-core/blob/main/aptos-move/move-examples/messageboard/sources/ACLMessageBoard.move) is similar to the capability based messageboard. The main difference is it uses ACL to control the access, which is essentially a list of the addresses. 

To add a new participant, the board creator adds the participant into the ACL list under his/her account. 

```tsx
public(script) fun add_participant(account: &signer, participant: address) acquires ACLBasedMB {
    let board = borrow_global_mut<ACLBasedMB>(Signer::address_of(account));
    ACL::add(&mut board.participants, participant);
}
```

When a participant wants to change the pinned message, the participant must first borrow the board resource under creator’s account, and check if the participant is in the ACL. Then the participant borrows mutable board to modify the `pinned_post`.
```tsx
public(script) fun send_pinned_message(
    account: &signer, board_addr: address, message: vector<u8>
) acquires ACLBasedMB, MessageChangeEventHandle {
    let board = borrow_global<ACLBasedMB>(board_addr);
    assert!(ACL::contains(&board.participants, Signer::address_of(account)),EACCOUNT_NOT_IN_ACL);

    let board = borrow_global_mut<ACLBasedMB>(board_addr);
    board.pinned_post = message;
```

## Interacting With Messageboard

### **Compile and Publish the Modules**

The [following code](https://github.com/aptos-labs/aptos-core/blob/main/developer-docs-site/static/messageboard_demo/client/index.ts) runs a command to compile the messageboard contract with the account address of developer. 

It then reads the compiled modules in `.mv` files in the module path and creates transactions to publish the modules under the developer’s account address.

```tsx
chdir('../../../../aptos-move/move-examples/messageboard');
execSync(
    `aptos move compile --package-dir . --named-addresses MessageBoard=${messageboardDev.address().toString()}`
);
var module_path = "build/MessageBoard";
MessageboardUtil.installMessageboard(client, messageboardDev, module_path)
```

### **Interact with the Messageboard**

We use [Aptos SDK](https://github.com/aptos-labs/aptos-core/tree/main/ecosystem/typescript/sdk) to interact with Aptos Blockchain. 

After the modules are published under the developer’s account address, we can use the functions of these published modules to specify the function name and argument to create the messageboard initialization payload, and send the transaction to devnet to create the messageboard.
```tsx
var fname = `${this.contractAddr.toString()}::${this.boardType}::message_board_init`;
var args = [];
const initPayload = MessageboardUtil.getScriptFunctionTxnPayload(fname, args);
await MessageboardUtil.executeTransaction(this.client, account, initPayload);
```

We can sendMessage to a messageboard by composing transactions with the send_message_to payload.

```tsx
async sendMessage(account: AptosAccount, message: string) {
    var hexstring = Buffer.from(message).toString('hex');
    var args = [this.adminAddr.toString(), hexstring];
    var fname = `${this.contractAddr.toString()}::${this.boardType}::send_message_to`;
    await MessageboardUtil.executeTransaction(this.client,
        account,
        MessageboardUtil.getScriptFunctionTxnPayload(fname, args)
    );
}
```

We can also read the events by querying the events published to an account. By default, we return all the events. We can record the sequence number of last event we received. Hence we can read all the new events sent after our last recorded event.
```tsx
async getLatestBoardEvents(): Promise<Types.Event[]> {
    var eventHandle = `${this.contractAddr.toString()}::${this.boardType}::MessageChangeEventHandle`;
    // get the latest page of events
    const params = {"start": this.latestEvent};
    const resp = await this.client.getEventsByEventHandle(
        this.adminAddr.toString(), eventHandle, 'change_events', params);
    // record the last event seen
    this.latestEvent = +resp[resp.length - 1].sequence_number;
    return resp
}
```