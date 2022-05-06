// This demonstrates a basic lifecycle of interacting with Messageboard Modules including
// 3 different roles: smart constractor developer, messageboard creator and participant
// 1. compiling and publish the modules with the developer's account address
// 2. creating messageboard with a creator account
// 3. sending txn to the messageboard using creator / participant's account
// 4. reading the events emitted from the messageboard

import {AptosClient, AptosAccount, FaucetClient, Types, HexString} from 'aptos';
import * as fs from 'fs';
import * as path from 'path';
import {Buffer} from 'buffer';
import {execSync} from "child_process";
import {chdir, cwd} from 'process';

const NODE_URL = "https://fullnode.devnet.aptoslabs.com";
const FAUCET_URL = "https://faucet.devnet.aptoslabs.com";

enum BoardType {
    ACL = "ACLBasedMB",
    CAP = "CapBasedMB"
}

class MessageboardUtil {

    // generate txn payload for any script function
    static getScriptFunctionTxnPayload(funcName: string, args: Types.MoveValue[]): Types.TransactionPayload {
        const payload = {
            type: "script_function_payload",
            function: `${funcName}`,
            type_arguments: [],
            arguments: args
        };
        return payload;
    }

    // exec a transaction
    static async executeTransaction(
        client: AptosClient,
        account: AptosAccount,
        payload: Types.TransactionPayload
    ): Promise<Types.HexEncodedBytes> {
        var txnRequest = await client.generateTransaction(account.address(), payload);
        var signedTxn = await client.signTransaction(account, txnRequest);
        var transactionRes = await client.submitTransaction(signedTxn);
        await client.waitForTransaction(transactionRes.hash);
        return transactionRes.hash;
    }

    // read the compiled mv move modules from specified path
    static getCompiledModuleTxnPayload(dirPath: string, names: Set<string>): Types.TransactionPayload {
        let files = [];

        function visit_dir(d: string) {
            fs.readdirSync(d).forEach(f => {
                const a = path.join(d, f);
                if (fs.statSync(a).isDirectory()) return visit_dir(a);
                else {
                    if (names.has(path.basename(a))) {
                        files.push(a);
                    }
                    return;
                }
            });
        }

        visit_dir(dirPath);
        console.log(files);
        let modules = files.map(function (e) {
            const moduleHex = fs.readFileSync(e).toString("hex");
            return {"bytecode": `0x${moduleHex}`}
        });

        const payload: Types.TransactionPayload = {
            type: "module_bundle_payload",
            modules: modules,
        };
        return payload;
    }

    // publish the compiled modules to aptos devnet
    static async installMessageboard(client: AptosClient, account: AptosAccount, modulePath: string) {
        console.log("Install messageboard");
        // publish the messageboard modules
        var boardModules = new Set<string>();
        boardModules.add("ACLBasedMB.mv");
        boardModules.add("CapBasedMB.mv");
        const boardModulePayload = MessageboardUtil.getCompiledModuleTxnPayload(modulePath, boardModules);
        const hash = await MessageboardUtil.executeTransaction(client, account, boardModulePayload);
        console.log(await client.getTransaction(hash));
    }
}

class Messageboard {
    boardType: BoardType;
    client: AptosClient;
    role: string;
    adminAddr: HexString;
    admin: AptosAccount;
    contractAddr: HexString;
    participants: Set<string>;
    latestEvent: number;
    claimedMessageCap: boolean; // this is only need for CAPMessageBoard

    constructor(client: AptosClient, boardType: BoardType, admin: AptosAccount, contractAddr: HexString) {
        this.boardType = boardType;
        this.client = client;
        this.admin = admin;
        this.adminAddr = admin.address();
        this.participants = new Set<string>();
        this.contractAddr = contractAddr;
        this.latestEvent = 0;
        this.claimedMessageCap = false;
    }

    async createMessageboard(account: AptosAccount) {
        console.log("creating message board");

        var fname = `${this.contractAddr.toString()}::${this.boardType}::message_board_init`;
        var args = [];
        const initPayload = MessageboardUtil.getScriptFunctionTxnPayload(fname, args);
        await MessageboardUtil.executeTransaction(this.client, account, initPayload);
    }

    async sendMessage(account: AptosAccount, message: string) {
        var hexstring = Buffer.from(message).toString('hex');
        console.log(`${account.address().toString()} sends message ${message} in the format of hex ${hexstring}`);

        var args = [this.adminAddr.toString(), hexstring];
        var fname = `${this.contractAddr.toString()}::${this.boardType}::send_message_to`;
        await MessageboardUtil.executeTransaction(this.client,
            account,
            MessageboardUtil.getScriptFunctionTxnPayload(fname, args)
        );
    }

    // different from sendMessage. this modify a pinned message resource stored on chain
    async sendPinnedMessage(account: AptosAccount, message: string) {
        var hexstring = Buffer.from(message).toString('hex');
        console.log(`${account.address().toString()} sends pinned message ${message} in the format of hex ${hexstring}`);

        var args = [this.adminAddr.toString(), hexstring];
        var fname = `${this.contractAddr.toString()}::${this.boardType}::send_pinned_message`;

        // verified if participant has capability to post message
        if (this.boardType == BoardType.CAP && !this.claimedMessageCap) {
            var cap_args = [this.adminAddr.toString()];
            var cap_fname = `${this.contractAddr.toString()}::${this.boardType}::claim_notice_cap`;
            await MessageboardUtil.executeTransaction(this.client,
                account,
                MessageboardUtil.getScriptFunctionTxnPayload(cap_fname, cap_args)
            );
            const res = await this.client.getAccountResources(account.address());
            console.log(res);
            const cap = res.find(
                (r) =>
                    r.type === `${this.contractAddr.toString()}::${this.boardType}::MessageChangeCapability` &&
                    "board" in r.data
            );
            if (cap !== null && cap.data['board'] === this.adminAddr.toString()) {
                this.claimedMessageCap = true;
            }
        }
        await MessageboardUtil.executeTransaction(this.client,
            account,
            MessageboardUtil.getScriptFunctionTxnPayload(fname, args)
        );
    }

    async viewMessageboardResource(): Promise<Types.AccountResource> {
        const res = await this.client.getAccountResources(this.adminAddr)
        console.log(res);
        const accountResource = res.find(
            (r) =>
                r.type === `${this.contractAddr.toString()}::${this.boardType}::${this.boardType}`
        );
        return accountResource;
    }

    async getLatestBoardEvents(): Promise<Types.Event[]> {
        console.log("getting latest events from messageboard");
        var eventHandle = `${this.contractAddr.toString()}::${this.boardType}::MessageChangeEventHandle`;

        // get the latest page of events
        const params = {"start": this.latestEvent};

        const resp = await this.client.getEventsByEventHandle(
            this.adminAddr.toString(), eventHandle, 'change_events', params);
        // record the last event seen
        this.latestEvent = +resp[resp.length - 1].sequence_number;
        return resp
    }

    async addParticipant(admin: AptosAccount, participant_addr: string) {
        console.log("add participants to messageboard");
        let args = [participant_addr];
        let fname = `${this.contractAddr.toString()}::${this.boardType}::add_participant`;
        const addParticipantPayload = MessageboardUtil.getScriptFunctionTxnPayload(fname, args);
        await MessageboardUtil.executeTransaction(this.client, admin, addParticipantPayload);
        this.participants.add(participant_addr);

    }

    async removeParticipant(admin: AptosAccount, participant_addr: string) {
        console.log("remove participant from messageboard");
        let args = [participant_addr];
        let fname = `${this.contractAddr.toString()}::${this.boardType}::remove_participant`;
        const removeParticipantPayload = MessageboardUtil.getScriptFunctionTxnPayload(fname, args);
        await MessageboardUtil.executeTransaction(this.client, admin, removeParticipantPayload);
        this.participants.delete(participant_addr);
    }
}

(async () => {
    const client = new AptosClient(NODE_URL);
    const faucetClient = new FaucetClient(NODE_URL, FAUCET_URL, null);

    // A smart contract developer compile and publish the modules
    var fakeKey = [1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 2];
    var messageboardDev = new AptosAccount(new Uint8Array(fakeKey));
    await faucetClient.fundAccount(messageboardDev.address(), 5000);

    console.log("publishing the messageboard constract to ", messageboardDev.address().hex());
    // compile the modules with the admin's address
    chdir('../../../../aptos-move/move-examples/messageboard');
    execSync(
        `aptos move compile --package-dir . --named-addresses MessageBoard=${messageboardDev.address().toString()}`
    );
    console.log("current directory: ", cwd());
    var module_path = "build/MessageBoard";
    MessageboardUtil.installMessageboard(client, messageboardDev, module_path)

    // The admin uses published contract to create their own messageboards
    var fakeKey1 = [1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 3];
    var boardAdmin = new AptosAccount(new Uint8Array(fakeKey1));
    var participant = new AptosAccount();
    await faucetClient.fundAccount(boardAdmin.address(), 5000);
    await faucetClient.fundAccount(participant.address(), 5000);

    var board = new Messageboard(client, BoardType.CAP, boardAdmin, messageboardDev.address());
    await board.createMessageboard(boardAdmin);

    await board.sendMessage(boardAdmin, "Hello World");
    console.log(await board.getLatestBoardEvents());

    // board admin can add participants to the board
    await board.addParticipant(boardAdmin, participant.address().toString());


    // participant of the board can send messages
    await board.sendMessage(participant, `Hey, I am ${participant.address().toString()}`);
    console.log(await board.getLatestBoardEvents());

    // participant with the authorization can modify the pinned message resource
    await board.sendPinnedMessage(participant, `Group Notice: Have a good day`);
    console.log(await board.getLatestBoardEvents());

    // anyone can view the resource under an account
    const aclboard = await board.viewMessageboardResource();
    console.log(aclboard);
})();
