export * from "../pkg/aptos_intent.js";

export async function get_wasm (){
   return (await import("../pkg/aptos_intent_bg.wasm")).default()
}