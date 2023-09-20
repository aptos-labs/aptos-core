import { AptosAccount } from "aptos";
import { execSync, spawnSync } from "child_process";

export class AptosCLI {

  static prepareNamedAddresses(namedAddresses: Map<string,AptosAccount>){
    const totalNames = namedAddresses.size;
    const newArgs: Array<string> = [];

    if (totalNames == 0){
      return newArgs
    }
      
    
    newArgs.push("--named-addresses");

    let idx= 0 ;
    namedAddresses.forEach((value, key) => {
      idx++;
      let toAppend = `${key}=${value.address().hex()}`;
      if (idx < totalNames - 1){
        toAppend += ","
      }
      newArgs.push(toAppend);
    });
    return newArgs;
  }


  static appendAdditionalArguments(additionalArguments: Map<string,string>){
    const totaArguments = additionalArguments.size;
    const newArgs: Array<string> = [];

    if (totaArguments == 0){
      return newArgs
    }

    let idx= 0 ;
    additionalArguments.forEach((value, key) => {
      idx++;
      let toAppend = `${key}=${value}`;
      if (idx < totaArguments - 1){
        toAppend += " "
      }
      newArgs.push(toAppend);
    });
    return newArgs;
  }

  // compile move modules
  async compilePackage(namedAddresses: Map<string,AptosAccount>, packageDir: string) {

    const args = ["aptos","move","compile","--save-metadata","--package-dir",packageDir,"--skip-fetch-latest-git-deps"]

    args.push(...AptosCLI.prepareNamedAddresses(namedAddresses))
    
    spawnSync(`npx`,args, {
      stdio: "inherit",
    });
  }
}
