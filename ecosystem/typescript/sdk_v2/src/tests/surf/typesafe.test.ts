import { Aptos } from "../../api";
import { AptosConfig } from "../../api/aptos_config";
import { Network } from "../../utils";
import { CoinAbi } from "./coinAbi";

describe("type-safe tests", () => {
    test("view function", () => {
        async () => {
            const settings: AptosConfig = { network: Network.DEVNET, faucet: Network.DEVNET }
            const aptos = new Aptos(settings);
            const client = aptos.general;

            // a view function call without type-safety
            client.view({
                function: "0x1::coin::balance",
                arguments: [],
                type_arguments: ["0x1"],
            });

            // a view function call with type-safety
            const [balance] = await client.view<CoinAbi, "balance">({
                function: "0x1::coin::balance",
                arguments: ["0x1"],
                type_arguments: ["0x1"],
            });

            // @ts-expect-error balance is a string instead of number
            const _double = balance * 2;

            // @ts-expect-error function name not exist
            client.view<CoinAbi, "baaaaa">({
                function: "0x1::coin::balance",
                arguments: ["0x1"],
                type_arguments: ["0x1"],
            });

            client.view<CoinAbi, "balance">({
                // @ts-expect-error function name is wrong
                function: "0x1::coin::baaaaaa",
                arguments: ["0x1"],
                type_arguments: ["0x1"],
            });

            // a view function call with type-safety
            client.view<CoinAbi, "balance">({
                function: "0x1::coin::balance",
                // @ts-expect-error need a address argument
                arguments: [123],
                type_arguments: ["0x1"],
            });

            // a view function call with type-safety
            client.view<CoinAbi, "balance">({
                function: "0x1::coin::balance",
                arguments: ["0x1"],
                // @ts-expect-error need only one type argument
                type_arguments: ["0x1", "0x1"],
            });
        }
    });
});
