import {
  Bytes,
  Deserializer,
  deserializeVector,
  Seq,
  Serializable,
  Serializer,
  serializeVector,
  Uint64,
  Uint8,
} from "../bcs";
import { AccountAddress } from "./account_address";

export class Any {
  constructor(public type_name: string, public data: Bytes) {}
  public serialize(serializer: Serializer): void {
    serializer.serializeStr(this.type_name);
    serializer.serializeBytes(this.data);
  }
  static deserialize(deserializer: Deserializer): Any {
    const type_name = deserializer.deserializeStr();
    const data = deserializer.deserializeBytes();
    return new Any(type_name, data);
  }
}

export class Option<T extends Serializable> {
  constructor(public vec: Seq<T>) {}
  public serialize(serializer: Serializer): void {
    serializeVector(this.vec, serializer);
  }
  static deserialize(deserializer: Deserializer): Option<any> {
    const vec = deserializeVector(deserializer, Any);
    return new Option(vec);
  }
}

export class UpgradePolicy {
  constructor(public policy: Uint8) {}
  public serialize(serializer: Serializer): void {
    serializer.serializeU8(this.policy);
  }
  public static deserialize(deserializer: Deserializer): UpgradePolicy {
    const policy = deserializer.deserializeU8();
    return new UpgradePolicy(policy);
  }
}

export class ModuleMetadata {
  constructor(public name: string, public source: Bytes, public source_map: Bytes, public extension: Option<Any>) {}
  public serialize(serializer: Serializer): void {
    serializer.serializeStr(this.name);
    serializer.serializeBytes(this.source);
    serializer.serializeBytes(this.source_map);
    this.extension.serialize(serializer);
  }
  public static deserialize(deserializer: Deserializer): ModuleMetadata {
    const name = deserializer.deserializeStr();
    const source = deserializer.deserializeBytes();
    const source_map = deserializer.deserializeBytes();
    const extension = Option.deserialize(deserializer);
    return new ModuleMetadata(name, source, source_map, extension);
  }
}

export class PackageDep {
  constructor(public account: AccountAddress, public package_name: string) {}
  public serialize(serializer: Serializer): void {
    this.account.serialize(serializer);
    serializer.serializeStr(this.package_name);
  }

  static deserialize(deserializer: Deserializer): PackageDep {
    const account = AccountAddress.deserialize(deserializer);
    const package_name = deserializer.deserializeStr();
    return new PackageDep(account, package_name);
  }
}

export class PackageMetadata {
  constructor(
    public name: string,
    public upgrade_policy: UpgradePolicy,
    public upgrade_number: Uint64,
    public source_digest: string,
    public manifest: Bytes,
    public modules: Seq<ModuleMetadata>,
    public deps: Seq<PackageDep>,
    public extension: Option<Any>,
  ) {}

  public serialize(serializer: Serializer): void {
    serializer.serializeStr(this.name);
    this.upgrade_policy.serialize(serializer);
    serializer.serializeU64(this.upgrade_number);
    serializer.serializeStr(this.source_digest);
    serializer.serializeBytes(this.manifest);
    serializeVector(this.modules, serializer);
    serializeVector(this.deps, serializer);
    this.extension.serialize(serializer);
  }

  static deserialize(deserializer: Deserializer): PackageMetadata {
    const value = deserializer.deserializeStr();
    const upgrade_policy = UpgradePolicy.deserialize(deserializer);
    const upgrade_number = deserializer.deserializeU64();
    const source_digest = deserializer.deserializeStr();
    const manifest = deserializer.deserializeBytes();
    const modules = deserializeVector(deserializer, ModuleMetadata);
    const deps = deserializeVector(deserializer, PackageDep);
    const extension = Option.deserialize(deserializer);
    return new PackageMetadata(
      value,
      upgrade_policy,
      upgrade_number,
      source_digest,
      manifest,
      modules,
      deps,
      extension,
    );
  }
}
