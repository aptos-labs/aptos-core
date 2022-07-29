// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

/* eslint-disable quotes */
/* eslint-disable max-len */

import { HexString } from "aptos";
import {
  ArgumentABI,
  ScriptFunctionABI,
  TypeTag,
  TypeTagAddress,
  TypeTagBool,
  TypeTagU128,
  TypeTagU64,
  TypeTagU8,
  TypeTagVector,
} from "aptos/dist/transaction_builder/aptos_types";
import { camelCase, pascalCase } from "change-case";
import invariant from "tiny-invariant";
import { MapWithDefault } from "../utils";

// A simple IR for code generation

export abstract class IRObject {
  protected lines: string[] = [];

  protected currentIndent: string = "";

  abstract gen(): string[];

  // Emit a block of code
  protected emitBlock(block: string) {
    let lines = block.split("\n");
    // We want to trim all the lines by the same number of leading spaces.
    // Count the leading spaces of the non-empty lines.
    const leadingSpaces = lines
      .filter((l) => l.trim().length > 0)
      .reduce((min, ln) => {
        let spaces = 0;
        for (; spaces < ln.length; spaces += 1) {
          if (ln[spaces] !== " ") {
            break;
          }
        }

        return Math.min(min, spaces);
      }, block.length);

    // Trim the lines
    lines = lines.map((l) => l.substring(leadingSpaces));

    lines.forEach((l) => this.emitln(l));
  }

  protected emitln(line: string) {
    this.lines.push(this.currentIndent + line);
  }

  protected getLines(): string[] {
    return this.lines;
  }

  protected indent() {
    this.currentIndent = `${this.currentIndent}  `;
  }

  protected unindent() {
    this.currentIndent = this.currentIndent.length < 2 ? "" : this.currentIndent.substring(2);
  }
}

export enum ImportType {
  BCS,
  TRANSACTION_BUILDER_TYPES,
  APTOS_ACCOUNT,
  APTOS_CLIENT,
  MAYBE_HEX_STRING,
  HEX_STRING,
  TYPE_TAG_PARSER,
}

export class MoveModule extends IRObject {
  private imports: Imports;

  private entryFunctions: EntryFunction[] = [];

  constructor(private readonly address: HexString, private readonly module: string) {
    super();
    this.imports = new Imports(new Set());
    this.imports.addImport(ImportType.APTOS_ACCOUNT);
    this.imports.addImport(ImportType.APTOS_CLIENT);
    this.imports.addImport(ImportType.BCS);
    this.imports.addImport(ImportType.TRANSACTION_BUILDER_TYPES);
  }

  moduleFullName(): string {
    return `${this.address.hex()}::${this.module}`;
  }

  addEntryFunction(f: EntryFunction) {
    this.entryFunctions.push(f);
  }

  gen(): string[] {
    // eslint-disable-next-line prefer-destructuring
    const imports = this.imports;

    const entryFuncLines = this.entryFunctions.map((ef) => ef.gen());

    this.entryFunctions.forEach((ef) => {
      imports.mergeImports(ef.imports);
    });

    // Generate imports
    imports.gen().forEach((l) => this.emitln(l));

    // Generate type aliases
    const typeArgNames = new Set();
    this.entryFunctions.forEach((ef) => {
      ef.typeTags.forEach((typeTag) => typeArgNames.add(typeTag));
    });
    typeArgNames.forEach((typeTag) => {
      this.emitln(`type ${typeTag} = string;`);
    });
    this.emitln("type TransactionHash = string;");
    this.emitln("");

    // Generate module class name
    this.emitln(`export class ${pascalCase(this.module)}`);
    this.emitln("{");
    this.indent();

    // Module fields
    this.emitln("private readonly moduleName: TxnBuilderTypes.ModuleId");

    // Contructor
    this.emitBlock(`
      constructor(
        private client: AptosClient,
        private sender: AptosAccount,
        private gasUnitPrice: BCS.Uint64 = 1n,
        private maxGasAmount: BCS.Uint64 = 1000n,
        private expSecFromNow: number = 10,
      ) {
        this.moduleName = new TxnBuilderTypes.ModuleId(
          TxnBuilderTypes.AccountAddress.fromHex("${this.address}"),
          new TxnBuilderTypes.Identifier("${this.module}"))
      }
    `);

    // Setters
    this.emitBlock(`
      setSender(sender: AptosAccount): ${pascalCase(this.module)} {
        this.sender = sender;
        return this;
      }

      setGasUnitPrice(gasUnitPrice: BCS.Uint64): ${pascalCase(this.module)} {
        this.gasUnitPrice = gasUnitPrice;
        return this;
      }

      setMaxGasAmount(maxGasAmount: BCS.Uint64): ${pascalCase(this.module)} {
        this.maxGasAmount = maxGasAmount;
        return this;
      }

      setExpSecFromNow(expSecFromNow: number): ${pascalCase(this.module)} {
        this.expSecFromNow = expSecFromNow;
        return this;
      }
    `);

    entryFuncLines.forEach((func) => {
      func.forEach((l) => this.emitln(l));
      this.emitln("");
    });
    this.unindent();
    this.emitln("}");

    return this.getLines();
  }
}

// <name, module>
type ImportTuple = [string, string];

class Imports extends IRObject {
  static IMPORT_MAP: Record<ImportType, ImportTuple> = {
    [ImportType.BCS]: ["BCS", "aptos"],
    [ImportType.TRANSACTION_BUILDER_TYPES]: ["TxnBuilderTypes", "aptos"],
    [ImportType.APTOS_ACCOUNT]: ["AptosAccount", "aptos"],
    [ImportType.APTOS_CLIENT]: ["AptosClient", "aptos"],
    [ImportType.MAYBE_HEX_STRING]: ["MaybeHexString", "aptos"],
    [ImportType.HEX_STRING]: ["HexString", "aptos"],
    [ImportType.TYPE_TAG_PARSER]: ["TypeTagParser", "aptos/dist/transaction_builder/builder_utils"],
  };

  constructor(private readonly imports: Set<ImportType>) {
    super();
  }

  addImport(importType: ImportType) {
    this.imports.add(importType);
  }

  mergeImports(impts: Imports) {
    impts.imports.forEach((impt) => {
      this.imports.add(impt);
    });
  }

  gen(): string[] {
    const compactedImportMap = new MapWithDefault<string, string[]>(() => []);
    this.imports.forEach((im) => {
      const [name, module] = Imports.IMPORT_MAP[im];
      compactedImportMap.get(module).push(name);
    });

    compactedImportMap.forEach((names, module) => {
      this.emitln(`import { ${names.sort().join(", ")} } from "${module}";`);
    });

    this.emitln("");

    return this.getLines();
  }
}

export class EntryFunction extends IRObject {
  // Dependencies of the function
  readonly imports: Imports;

  readonly typeTags: string[];

  constructor(private readonly abi: ScriptFunctionABI) {
    super();
    this.imports = new Imports(new Set());
    this.typeTags = abi.ty_args.map((ta) => pascalCase(ta.name));
    if (this.typeTags.length > 0) {
      this.imports.addImport(ImportType.TYPE_TAG_PARSER);
    }

    // Function always requires transaction builder types to build raw transactions.
    this.imports.addImport(ImportType.TRANSACTION_BUILDER_TYPES);

    // Function always requires AptosClient to submit txns.
    this.imports.addImport(ImportType.APTOS_CLIENT);
  }

  // eslint-disable-next-line consistent-return
  private typeTagToString(typeTag: TypeTag): string {
    if (typeTag instanceof TypeTagBool) {
      return "boolean";
    }

    if (typeTag instanceof TypeTagU8) {
      this.imports.addImport(ImportType.BCS);
      return "BCS.Uint8";
    }

    if (typeTag instanceof TypeTagU64) {
      this.imports.addImport(ImportType.BCS);
      return "BCS.Uint64";
    }

    if (typeTag instanceof TypeTagU128) {
      this.imports.addImport(ImportType.BCS);
      return "BCS.Uint128";
    }

    if (typeTag instanceof TypeTagAddress) {
      this.imports.addImport(ImportType.TRANSACTION_BUILDER_TYPES);
      return "TxnBuilderTypes.AccountAddress";
    }

    if (typeTag instanceof TypeTagVector) {
      this.imports.addImport(ImportType.BCS);
      const vecTag = typeTag as TypeTagVector;
      return `BCS.Seq<${this.typeTagToString(vecTag.value)}>`;
    }

    // Shouldn't be here
    invariant(false, "Unsupported type tag");
  }

  private genArgInSignature(arg: ArgumentABI): string {
    return `${camelCase(arg.name)}: ${this.typeTagToString(arg.type_tag)}`;
  }

  private genArgBCS(serializerName: string, argName: string, argType: TypeTag, level: number) {
    if (argType instanceof TypeTagBool) {
      this.emitln(`${serializerName}.serializeBool(${argName});`);
      this.emitln("");
    } else if (argType instanceof TypeTagU8) {
      this.emitln(`${serializerName}.serializeU8(${argName});`);
      this.emitln("");
    } else if (argType instanceof TypeTagU64) {
      this.emitln(`${serializerName}.serializeU64(${argName});`);
      this.emitln("");
    } else if (argType instanceof TypeTagU128) {
      this.emitln(`${serializerName}.serializeU128(${argName});`);
      this.emitln("");
    } else if (argType instanceof TypeTagAddress) {
      this.emitln(`${argName}.serialize(${serializerName});`);
      this.emitln("");
    } else if (argType instanceof TypeTagVector) {
      this.emitln(`${serializerName}.serializeU32AsUleb128(${argName}.length);`);
      this.emitln(`for (const ${argName}${level} of ${argName})`);
      this.emitln("{");
      this.indent();
      this.genArgBCS(serializerName, `${argName}${level}`, (argType as TypeTagVector).value, level + 1);
      this.unindent();
      this.emitln("}");
    } else {
      // Shouldn't be here
      invariant(false, "Unsupported arg type");
    }
  }

  gen(): string[] {
    // Because of the limitation of TypeScript generics, we have to ask users to pass in
    // the typeArgs as normal function args.
    let typeArgAsNormalArgs = "";

    if (this.typeTags.length > 0) {
      // typeArgs is defined as a tuple
      typeArgAsNormalArgs = `typeArgs: [${this.typeTags.join(", ")}], `;
    }

    // e.g. async transfer(to: TxnBuilderTypes.AccountAddress, amount: BCS.Uint64): string
    // Transaction hash string is returned
    const signature = `async ${camelCase(this.abi.name)}(${typeArgAsNormalArgs}${this.abi.args
      .map((arg) => this.genArgInSignature(arg))
      .join(", ")}): Promise<TransactionHash>`;

    this.emitln(signature);
    this.emitln("{");
    this.indent();

    // Serializes the args
    this.abi.args.forEach((arg) => {
      const serializerName = `${camelCase(arg.name)}Serializer`;
      this.emitln(`const ${serializerName} = new BCS.Serializer();`);
      this.genArgBCS(serializerName, camelCase(arg.name), arg.type_tag, 0);
    });

    // Prepares the type tags
    this.emitln("const parsedTypeTags: TxnBuilderTypes.TypeTag[] = [];");

    if (this.typeTags.length > 0) {
      this.emitBlock(`
        if (typeArgs.length !== ${this.typeTags.length}) {
          throw new Error("typeArgs length should be ${this.typeTags.length}");
        }
      `);
    }

    for (let i = 0; i < this.typeTags.length; i += 1) {
      this.emitBlock(`
        const tyTagParser${i} = new TypeTagParser(typeArgs[${i}]);
        parsedTypeTags.push(tyTagParser${i}.parseTypeTag());
      `);
    }

    this.emitBlock(`
      const payload = new TxnBuilderTypes.TransactionPayloadScriptFunction(
        new TxnBuilderTypes.ScriptFunction(
          this.moduleName,
          new TxnBuilderTypes.Identifier("${this.abi.name}"),
          parsedTypeTags,
          [${this.abi.args.map((arg) => `${camelCase(arg.name)}Serializer.getBytes()`).join(", ")}]
        )
      );

      if (!this.sender) {
        throw new Error("Transaction sender is not found.");
      }

      const [{ sequence_number: sequenceNumber }, chainId] = await Promise.all([
        this.client.getAccount(this.sender.address()),
        this.client.getChainId(),
      ]);

      const rawTxn = new TxnBuilderTypes.RawTransaction(
        TxnBuilderTypes.AccountAddress.fromHex(this.sender.address()),
        BigInt(sequenceNumber),
        payload,
        BigInt(this.maxGasAmount),
        BigInt(this.gasUnitPrice),
        BigInt(Math.floor(Date.now() / 1000) + this.expSecFromNow),
        new TxnBuilderTypes.ChainId(chainId),
      );

      const bcsTxn = AptosClient.generateBCSTransaction(this.sender, rawTxn);
      const { hash } = await this.client.submitSignedBCSTransaction(bcsTxn);
      return hash;
    `);

    this.unindent();
    this.emitln("}");

    return this.getLines();
  }
}
