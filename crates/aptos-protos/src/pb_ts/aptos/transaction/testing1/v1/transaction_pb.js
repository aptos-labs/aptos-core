// source: aptos/transaction/testing1/v1/transaction.proto
/**
 * @fileoverview
 * @enhanceable
 * @suppress {missingRequire} reports error on implicit type usages.
 * @suppress {messageConventions} JS Compiler reports an error if a variable or
 *     field starts with 'MSG_' and isn't a translatable message.
 * @public
 */
// GENERATED CODE -- DO NOT EDIT!
/* eslint-disable */
// @ts-nocheck

var jspb = require('google-protobuf');
var goog = jspb;
var global = (function() { return this || window || global || self || Function('return this')(); }).call(null);

var aptos_util_timestamp_timestamp_pb = require('../../../../aptos/util/timestamp/timestamp_pb.js');
goog.object.extend(proto, aptos_util_timestamp_timestamp_pb);
goog.exportSymbol('proto.aptos.transaction.testing1.v1.AccountSignature', null, global);
goog.exportSymbol('proto.aptos.transaction.testing1.v1.AccountSignature.SignatureCase', null, global);
goog.exportSymbol('proto.aptos.transaction.testing1.v1.AccountSignature.Type', null, global);
goog.exportSymbol('proto.aptos.transaction.testing1.v1.Block', null, global);
goog.exportSymbol('proto.aptos.transaction.testing1.v1.BlockMetadataTransaction', null, global);
goog.exportSymbol('proto.aptos.transaction.testing1.v1.DeleteModule', null, global);
goog.exportSymbol('proto.aptos.transaction.testing1.v1.DeleteResource', null, global);
goog.exportSymbol('proto.aptos.transaction.testing1.v1.DeleteTableData', null, global);
goog.exportSymbol('proto.aptos.transaction.testing1.v1.DeleteTableItem', null, global);
goog.exportSymbol('proto.aptos.transaction.testing1.v1.DirectWriteSet', null, global);
goog.exportSymbol('proto.aptos.transaction.testing1.v1.Ed25519Signature', null, global);
goog.exportSymbol('proto.aptos.transaction.testing1.v1.EntryFunctionId', null, global);
goog.exportSymbol('proto.aptos.transaction.testing1.v1.EntryFunctionPayload', null, global);
goog.exportSymbol('proto.aptos.transaction.testing1.v1.Event', null, global);
goog.exportSymbol('proto.aptos.transaction.testing1.v1.EventKey', null, global);
goog.exportSymbol('proto.aptos.transaction.testing1.v1.GenesisTransaction', null, global);
goog.exportSymbol('proto.aptos.transaction.testing1.v1.ModuleBundlePayload', null, global);
goog.exportSymbol('proto.aptos.transaction.testing1.v1.MoveAbility', null, global);
goog.exportSymbol('proto.aptos.transaction.testing1.v1.MoveFunction', null, global);
goog.exportSymbol('proto.aptos.transaction.testing1.v1.MoveFunction.Visibility', null, global);
goog.exportSymbol('proto.aptos.transaction.testing1.v1.MoveFunctionGenericTypeParam', null, global);
goog.exportSymbol('proto.aptos.transaction.testing1.v1.MoveModule', null, global);
goog.exportSymbol('proto.aptos.transaction.testing1.v1.MoveModuleBytecode', null, global);
goog.exportSymbol('proto.aptos.transaction.testing1.v1.MoveModuleId', null, global);
goog.exportSymbol('proto.aptos.transaction.testing1.v1.MoveScriptBytecode', null, global);
goog.exportSymbol('proto.aptos.transaction.testing1.v1.MoveStruct', null, global);
goog.exportSymbol('proto.aptos.transaction.testing1.v1.MoveStructField', null, global);
goog.exportSymbol('proto.aptos.transaction.testing1.v1.MoveStructGenericTypeParam', null, global);
goog.exportSymbol('proto.aptos.transaction.testing1.v1.MoveStructTag', null, global);
goog.exportSymbol('proto.aptos.transaction.testing1.v1.MoveType', null, global);
goog.exportSymbol('proto.aptos.transaction.testing1.v1.MoveType.ContentCase', null, global);
goog.exportSymbol('proto.aptos.transaction.testing1.v1.MoveType.ReferenceType', null, global);
goog.exportSymbol('proto.aptos.transaction.testing1.v1.MoveTypes', null, global);
goog.exportSymbol('proto.aptos.transaction.testing1.v1.MultiAgentSignature', null, global);
goog.exportSymbol('proto.aptos.transaction.testing1.v1.MultiEd25519Signature', null, global);
goog.exportSymbol('proto.aptos.transaction.testing1.v1.MultisigPayload', null, global);
goog.exportSymbol('proto.aptos.transaction.testing1.v1.MultisigTransactionPayload', null, global);
goog.exportSymbol('proto.aptos.transaction.testing1.v1.MultisigTransactionPayload.PayloadCase', null, global);
goog.exportSymbol('proto.aptos.transaction.testing1.v1.MultisigTransactionPayload.Type', null, global);
goog.exportSymbol('proto.aptos.transaction.testing1.v1.ScriptPayload', null, global);
goog.exportSymbol('proto.aptos.transaction.testing1.v1.ScriptWriteSet', null, global);
goog.exportSymbol('proto.aptos.transaction.testing1.v1.Signature', null, global);
goog.exportSymbol('proto.aptos.transaction.testing1.v1.Signature.SignatureCase', null, global);
goog.exportSymbol('proto.aptos.transaction.testing1.v1.Signature.Type', null, global);
goog.exportSymbol('proto.aptos.transaction.testing1.v1.StateCheckpointTransaction', null, global);
goog.exportSymbol('proto.aptos.transaction.testing1.v1.Transaction', null, global);
goog.exportSymbol('proto.aptos.transaction.testing1.v1.Transaction.TransactionType', null, global);
goog.exportSymbol('proto.aptos.transaction.testing1.v1.Transaction.TxnDataCase', null, global);
goog.exportSymbol('proto.aptos.transaction.testing1.v1.TransactionInfo', null, global);
goog.exportSymbol('proto.aptos.transaction.testing1.v1.TransactionPayload', null, global);
goog.exportSymbol('proto.aptos.transaction.testing1.v1.TransactionPayload.PayloadCase', null, global);
goog.exportSymbol('proto.aptos.transaction.testing1.v1.TransactionPayload.Type', null, global);
goog.exportSymbol('proto.aptos.transaction.testing1.v1.UserTransaction', null, global);
goog.exportSymbol('proto.aptos.transaction.testing1.v1.UserTransactionRequest', null, global);
goog.exportSymbol('proto.aptos.transaction.testing1.v1.WriteModule', null, global);
goog.exportSymbol('proto.aptos.transaction.testing1.v1.WriteResource', null, global);
goog.exportSymbol('proto.aptos.transaction.testing1.v1.WriteSet', null, global);
goog.exportSymbol('proto.aptos.transaction.testing1.v1.WriteSet.WriteSetCase', null, global);
goog.exportSymbol('proto.aptos.transaction.testing1.v1.WriteSet.WriteSetType', null, global);
goog.exportSymbol('proto.aptos.transaction.testing1.v1.WriteSetChange', null, global);
goog.exportSymbol('proto.aptos.transaction.testing1.v1.WriteSetChange.ChangeCase', null, global);
goog.exportSymbol('proto.aptos.transaction.testing1.v1.WriteSetChange.Type', null, global);
goog.exportSymbol('proto.aptos.transaction.testing1.v1.WriteSetPayload', null, global);
goog.exportSymbol('proto.aptos.transaction.testing1.v1.WriteTableData', null, global);
goog.exportSymbol('proto.aptos.transaction.testing1.v1.WriteTableItem', null, global);
/**
 * Generated by JsPbCodeGenerator.
 * @param {Array=} opt_data Optional initial data array, typically from a
 * server response, or constructed directly in Javascript. The array is used
 * in place and becomes part of the constructed object. It is not cloned.
 * If no data is provided, the constructed object will be empty, but still
 * valid.
 * @extends {jspb.Message}
 * @constructor
 */
proto.aptos.transaction.testing1.v1.Block = function(opt_data) {
  jspb.Message.initialize(this, opt_data, 0, -1, proto.aptos.transaction.testing1.v1.Block.repeatedFields_, null);
};
goog.inherits(proto.aptos.transaction.testing1.v1.Block, jspb.Message);
if (goog.DEBUG && !COMPILED) {
  /**
   * @public
   * @override
   */
  proto.aptos.transaction.testing1.v1.Block.displayName = 'proto.aptos.transaction.testing1.v1.Block';
}
/**
 * Generated by JsPbCodeGenerator.
 * @param {Array=} opt_data Optional initial data array, typically from a
 * server response, or constructed directly in Javascript. The array is used
 * in place and becomes part of the constructed object. It is not cloned.
 * If no data is provided, the constructed object will be empty, but still
 * valid.
 * @extends {jspb.Message}
 * @constructor
 */
proto.aptos.transaction.testing1.v1.Transaction = function(opt_data) {
  jspb.Message.initialize(this, opt_data, 0, -1, null, proto.aptos.transaction.testing1.v1.Transaction.oneofGroups_);
};
goog.inherits(proto.aptos.transaction.testing1.v1.Transaction, jspb.Message);
if (goog.DEBUG && !COMPILED) {
  /**
   * @public
   * @override
   */
  proto.aptos.transaction.testing1.v1.Transaction.displayName = 'proto.aptos.transaction.testing1.v1.Transaction';
}
/**
 * Generated by JsPbCodeGenerator.
 * @param {Array=} opt_data Optional initial data array, typically from a
 * server response, or constructed directly in Javascript. The array is used
 * in place and becomes part of the constructed object. It is not cloned.
 * If no data is provided, the constructed object will be empty, but still
 * valid.
 * @extends {jspb.Message}
 * @constructor
 */
proto.aptos.transaction.testing1.v1.BlockMetadataTransaction = function(opt_data) {
  jspb.Message.initialize(this, opt_data, 0, -1, proto.aptos.transaction.testing1.v1.BlockMetadataTransaction.repeatedFields_, null);
};
goog.inherits(proto.aptos.transaction.testing1.v1.BlockMetadataTransaction, jspb.Message);
if (goog.DEBUG && !COMPILED) {
  /**
   * @public
   * @override
   */
  proto.aptos.transaction.testing1.v1.BlockMetadataTransaction.displayName = 'proto.aptos.transaction.testing1.v1.BlockMetadataTransaction';
}
/**
 * Generated by JsPbCodeGenerator.
 * @param {Array=} opt_data Optional initial data array, typically from a
 * server response, or constructed directly in Javascript. The array is used
 * in place and becomes part of the constructed object. It is not cloned.
 * If no data is provided, the constructed object will be empty, but still
 * valid.
 * @extends {jspb.Message}
 * @constructor
 */
proto.aptos.transaction.testing1.v1.GenesisTransaction = function(opt_data) {
  jspb.Message.initialize(this, opt_data, 0, -1, proto.aptos.transaction.testing1.v1.GenesisTransaction.repeatedFields_, null);
};
goog.inherits(proto.aptos.transaction.testing1.v1.GenesisTransaction, jspb.Message);
if (goog.DEBUG && !COMPILED) {
  /**
   * @public
   * @override
   */
  proto.aptos.transaction.testing1.v1.GenesisTransaction.displayName = 'proto.aptos.transaction.testing1.v1.GenesisTransaction';
}
/**
 * Generated by JsPbCodeGenerator.
 * @param {Array=} opt_data Optional initial data array, typically from a
 * server response, or constructed directly in Javascript. The array is used
 * in place and becomes part of the constructed object. It is not cloned.
 * If no data is provided, the constructed object will be empty, but still
 * valid.
 * @extends {jspb.Message}
 * @constructor
 */
proto.aptos.transaction.testing1.v1.StateCheckpointTransaction = function(opt_data) {
  jspb.Message.initialize(this, opt_data, 0, -1, null, null);
};
goog.inherits(proto.aptos.transaction.testing1.v1.StateCheckpointTransaction, jspb.Message);
if (goog.DEBUG && !COMPILED) {
  /**
   * @public
   * @override
   */
  proto.aptos.transaction.testing1.v1.StateCheckpointTransaction.displayName = 'proto.aptos.transaction.testing1.v1.StateCheckpointTransaction';
}
/**
 * Generated by JsPbCodeGenerator.
 * @param {Array=} opt_data Optional initial data array, typically from a
 * server response, or constructed directly in Javascript. The array is used
 * in place and becomes part of the constructed object. It is not cloned.
 * If no data is provided, the constructed object will be empty, but still
 * valid.
 * @extends {jspb.Message}
 * @constructor
 */
proto.aptos.transaction.testing1.v1.UserTransaction = function(opt_data) {
  jspb.Message.initialize(this, opt_data, 0, -1, proto.aptos.transaction.testing1.v1.UserTransaction.repeatedFields_, null);
};
goog.inherits(proto.aptos.transaction.testing1.v1.UserTransaction, jspb.Message);
if (goog.DEBUG && !COMPILED) {
  /**
   * @public
   * @override
   */
  proto.aptos.transaction.testing1.v1.UserTransaction.displayName = 'proto.aptos.transaction.testing1.v1.UserTransaction';
}
/**
 * Generated by JsPbCodeGenerator.
 * @param {Array=} opt_data Optional initial data array, typically from a
 * server response, or constructed directly in Javascript. The array is used
 * in place and becomes part of the constructed object. It is not cloned.
 * If no data is provided, the constructed object will be empty, but still
 * valid.
 * @extends {jspb.Message}
 * @constructor
 */
proto.aptos.transaction.testing1.v1.Event = function(opt_data) {
  jspb.Message.initialize(this, opt_data, 0, -1, null, null);
};
goog.inherits(proto.aptos.transaction.testing1.v1.Event, jspb.Message);
if (goog.DEBUG && !COMPILED) {
  /**
   * @public
   * @override
   */
  proto.aptos.transaction.testing1.v1.Event.displayName = 'proto.aptos.transaction.testing1.v1.Event';
}
/**
 * Generated by JsPbCodeGenerator.
 * @param {Array=} opt_data Optional initial data array, typically from a
 * server response, or constructed directly in Javascript. The array is used
 * in place and becomes part of the constructed object. It is not cloned.
 * If no data is provided, the constructed object will be empty, but still
 * valid.
 * @extends {jspb.Message}
 * @constructor
 */
proto.aptos.transaction.testing1.v1.TransactionInfo = function(opt_data) {
  jspb.Message.initialize(this, opt_data, 0, -1, proto.aptos.transaction.testing1.v1.TransactionInfo.repeatedFields_, null);
};
goog.inherits(proto.aptos.transaction.testing1.v1.TransactionInfo, jspb.Message);
if (goog.DEBUG && !COMPILED) {
  /**
   * @public
   * @override
   */
  proto.aptos.transaction.testing1.v1.TransactionInfo.displayName = 'proto.aptos.transaction.testing1.v1.TransactionInfo';
}
/**
 * Generated by JsPbCodeGenerator.
 * @param {Array=} opt_data Optional initial data array, typically from a
 * server response, or constructed directly in Javascript. The array is used
 * in place and becomes part of the constructed object. It is not cloned.
 * If no data is provided, the constructed object will be empty, but still
 * valid.
 * @extends {jspb.Message}
 * @constructor
 */
proto.aptos.transaction.testing1.v1.EventKey = function(opt_data) {
  jspb.Message.initialize(this, opt_data, 0, -1, null, null);
};
goog.inherits(proto.aptos.transaction.testing1.v1.EventKey, jspb.Message);
if (goog.DEBUG && !COMPILED) {
  /**
   * @public
   * @override
   */
  proto.aptos.transaction.testing1.v1.EventKey.displayName = 'proto.aptos.transaction.testing1.v1.EventKey';
}
/**
 * Generated by JsPbCodeGenerator.
 * @param {Array=} opt_data Optional initial data array, typically from a
 * server response, or constructed directly in Javascript. The array is used
 * in place and becomes part of the constructed object. It is not cloned.
 * If no data is provided, the constructed object will be empty, but still
 * valid.
 * @extends {jspb.Message}
 * @constructor
 */
proto.aptos.transaction.testing1.v1.UserTransactionRequest = function(opt_data) {
  jspb.Message.initialize(this, opt_data, 0, -1, null, null);
};
goog.inherits(proto.aptos.transaction.testing1.v1.UserTransactionRequest, jspb.Message);
if (goog.DEBUG && !COMPILED) {
  /**
   * @public
   * @override
   */
  proto.aptos.transaction.testing1.v1.UserTransactionRequest.displayName = 'proto.aptos.transaction.testing1.v1.UserTransactionRequest';
}
/**
 * Generated by JsPbCodeGenerator.
 * @param {Array=} opt_data Optional initial data array, typically from a
 * server response, or constructed directly in Javascript. The array is used
 * in place and becomes part of the constructed object. It is not cloned.
 * If no data is provided, the constructed object will be empty, but still
 * valid.
 * @extends {jspb.Message}
 * @constructor
 */
proto.aptos.transaction.testing1.v1.WriteSet = function(opt_data) {
  jspb.Message.initialize(this, opt_data, 0, -1, null, proto.aptos.transaction.testing1.v1.WriteSet.oneofGroups_);
};
goog.inherits(proto.aptos.transaction.testing1.v1.WriteSet, jspb.Message);
if (goog.DEBUG && !COMPILED) {
  /**
   * @public
   * @override
   */
  proto.aptos.transaction.testing1.v1.WriteSet.displayName = 'proto.aptos.transaction.testing1.v1.WriteSet';
}
/**
 * Generated by JsPbCodeGenerator.
 * @param {Array=} opt_data Optional initial data array, typically from a
 * server response, or constructed directly in Javascript. The array is used
 * in place and becomes part of the constructed object. It is not cloned.
 * If no data is provided, the constructed object will be empty, but still
 * valid.
 * @extends {jspb.Message}
 * @constructor
 */
proto.aptos.transaction.testing1.v1.ScriptWriteSet = function(opt_data) {
  jspb.Message.initialize(this, opt_data, 0, -1, null, null);
};
goog.inherits(proto.aptos.transaction.testing1.v1.ScriptWriteSet, jspb.Message);
if (goog.DEBUG && !COMPILED) {
  /**
   * @public
   * @override
   */
  proto.aptos.transaction.testing1.v1.ScriptWriteSet.displayName = 'proto.aptos.transaction.testing1.v1.ScriptWriteSet';
}
/**
 * Generated by JsPbCodeGenerator.
 * @param {Array=} opt_data Optional initial data array, typically from a
 * server response, or constructed directly in Javascript. The array is used
 * in place and becomes part of the constructed object. It is not cloned.
 * If no data is provided, the constructed object will be empty, but still
 * valid.
 * @extends {jspb.Message}
 * @constructor
 */
proto.aptos.transaction.testing1.v1.DirectWriteSet = function(opt_data) {
  jspb.Message.initialize(this, opt_data, 0, -1, proto.aptos.transaction.testing1.v1.DirectWriteSet.repeatedFields_, null);
};
goog.inherits(proto.aptos.transaction.testing1.v1.DirectWriteSet, jspb.Message);
if (goog.DEBUG && !COMPILED) {
  /**
   * @public
   * @override
   */
  proto.aptos.transaction.testing1.v1.DirectWriteSet.displayName = 'proto.aptos.transaction.testing1.v1.DirectWriteSet';
}
/**
 * Generated by JsPbCodeGenerator.
 * @param {Array=} opt_data Optional initial data array, typically from a
 * server response, or constructed directly in Javascript. The array is used
 * in place and becomes part of the constructed object. It is not cloned.
 * If no data is provided, the constructed object will be empty, but still
 * valid.
 * @extends {jspb.Message}
 * @constructor
 */
proto.aptos.transaction.testing1.v1.WriteSetChange = function(opt_data) {
  jspb.Message.initialize(this, opt_data, 0, -1, null, proto.aptos.transaction.testing1.v1.WriteSetChange.oneofGroups_);
};
goog.inherits(proto.aptos.transaction.testing1.v1.WriteSetChange, jspb.Message);
if (goog.DEBUG && !COMPILED) {
  /**
   * @public
   * @override
   */
  proto.aptos.transaction.testing1.v1.WriteSetChange.displayName = 'proto.aptos.transaction.testing1.v1.WriteSetChange';
}
/**
 * Generated by JsPbCodeGenerator.
 * @param {Array=} opt_data Optional initial data array, typically from a
 * server response, or constructed directly in Javascript. The array is used
 * in place and becomes part of the constructed object. It is not cloned.
 * If no data is provided, the constructed object will be empty, but still
 * valid.
 * @extends {jspb.Message}
 * @constructor
 */
proto.aptos.transaction.testing1.v1.DeleteModule = function(opt_data) {
  jspb.Message.initialize(this, opt_data, 0, -1, null, null);
};
goog.inherits(proto.aptos.transaction.testing1.v1.DeleteModule, jspb.Message);
if (goog.DEBUG && !COMPILED) {
  /**
   * @public
   * @override
   */
  proto.aptos.transaction.testing1.v1.DeleteModule.displayName = 'proto.aptos.transaction.testing1.v1.DeleteModule';
}
/**
 * Generated by JsPbCodeGenerator.
 * @param {Array=} opt_data Optional initial data array, typically from a
 * server response, or constructed directly in Javascript. The array is used
 * in place and becomes part of the constructed object. It is not cloned.
 * If no data is provided, the constructed object will be empty, but still
 * valid.
 * @extends {jspb.Message}
 * @constructor
 */
proto.aptos.transaction.testing1.v1.DeleteResource = function(opt_data) {
  jspb.Message.initialize(this, opt_data, 0, -1, null, null);
};
goog.inherits(proto.aptos.transaction.testing1.v1.DeleteResource, jspb.Message);
if (goog.DEBUG && !COMPILED) {
  /**
   * @public
   * @override
   */
  proto.aptos.transaction.testing1.v1.DeleteResource.displayName = 'proto.aptos.transaction.testing1.v1.DeleteResource';
}
/**
 * Generated by JsPbCodeGenerator.
 * @param {Array=} opt_data Optional initial data array, typically from a
 * server response, or constructed directly in Javascript. The array is used
 * in place and becomes part of the constructed object. It is not cloned.
 * If no data is provided, the constructed object will be empty, but still
 * valid.
 * @extends {jspb.Message}
 * @constructor
 */
proto.aptos.transaction.testing1.v1.DeleteTableItem = function(opt_data) {
  jspb.Message.initialize(this, opt_data, 0, -1, null, null);
};
goog.inherits(proto.aptos.transaction.testing1.v1.DeleteTableItem, jspb.Message);
if (goog.DEBUG && !COMPILED) {
  /**
   * @public
   * @override
   */
  proto.aptos.transaction.testing1.v1.DeleteTableItem.displayName = 'proto.aptos.transaction.testing1.v1.DeleteTableItem';
}
/**
 * Generated by JsPbCodeGenerator.
 * @param {Array=} opt_data Optional initial data array, typically from a
 * server response, or constructed directly in Javascript. The array is used
 * in place and becomes part of the constructed object. It is not cloned.
 * If no data is provided, the constructed object will be empty, but still
 * valid.
 * @extends {jspb.Message}
 * @constructor
 */
proto.aptos.transaction.testing1.v1.DeleteTableData = function(opt_data) {
  jspb.Message.initialize(this, opt_data, 0, -1, null, null);
};
goog.inherits(proto.aptos.transaction.testing1.v1.DeleteTableData, jspb.Message);
if (goog.DEBUG && !COMPILED) {
  /**
   * @public
   * @override
   */
  proto.aptos.transaction.testing1.v1.DeleteTableData.displayName = 'proto.aptos.transaction.testing1.v1.DeleteTableData';
}
/**
 * Generated by JsPbCodeGenerator.
 * @param {Array=} opt_data Optional initial data array, typically from a
 * server response, or constructed directly in Javascript. The array is used
 * in place and becomes part of the constructed object. It is not cloned.
 * If no data is provided, the constructed object will be empty, but still
 * valid.
 * @extends {jspb.Message}
 * @constructor
 */
proto.aptos.transaction.testing1.v1.WriteModule = function(opt_data) {
  jspb.Message.initialize(this, opt_data, 0, -1, null, null);
};
goog.inherits(proto.aptos.transaction.testing1.v1.WriteModule, jspb.Message);
if (goog.DEBUG && !COMPILED) {
  /**
   * @public
   * @override
   */
  proto.aptos.transaction.testing1.v1.WriteModule.displayName = 'proto.aptos.transaction.testing1.v1.WriteModule';
}
/**
 * Generated by JsPbCodeGenerator.
 * @param {Array=} opt_data Optional initial data array, typically from a
 * server response, or constructed directly in Javascript. The array is used
 * in place and becomes part of the constructed object. It is not cloned.
 * If no data is provided, the constructed object will be empty, but still
 * valid.
 * @extends {jspb.Message}
 * @constructor
 */
proto.aptos.transaction.testing1.v1.WriteResource = function(opt_data) {
  jspb.Message.initialize(this, opt_data, 0, -1, null, null);
};
goog.inherits(proto.aptos.transaction.testing1.v1.WriteResource, jspb.Message);
if (goog.DEBUG && !COMPILED) {
  /**
   * @public
   * @override
   */
  proto.aptos.transaction.testing1.v1.WriteResource.displayName = 'proto.aptos.transaction.testing1.v1.WriteResource';
}
/**
 * Generated by JsPbCodeGenerator.
 * @param {Array=} opt_data Optional initial data array, typically from a
 * server response, or constructed directly in Javascript. The array is used
 * in place and becomes part of the constructed object. It is not cloned.
 * If no data is provided, the constructed object will be empty, but still
 * valid.
 * @extends {jspb.Message}
 * @constructor
 */
proto.aptos.transaction.testing1.v1.WriteTableData = function(opt_data) {
  jspb.Message.initialize(this, opt_data, 0, -1, null, null);
};
goog.inherits(proto.aptos.transaction.testing1.v1.WriteTableData, jspb.Message);
if (goog.DEBUG && !COMPILED) {
  /**
   * @public
   * @override
   */
  proto.aptos.transaction.testing1.v1.WriteTableData.displayName = 'proto.aptos.transaction.testing1.v1.WriteTableData';
}
/**
 * Generated by JsPbCodeGenerator.
 * @param {Array=} opt_data Optional initial data array, typically from a
 * server response, or constructed directly in Javascript. The array is used
 * in place and becomes part of the constructed object. It is not cloned.
 * If no data is provided, the constructed object will be empty, but still
 * valid.
 * @extends {jspb.Message}
 * @constructor
 */
proto.aptos.transaction.testing1.v1.WriteTableItem = function(opt_data) {
  jspb.Message.initialize(this, opt_data, 0, -1, null, null);
};
goog.inherits(proto.aptos.transaction.testing1.v1.WriteTableItem, jspb.Message);
if (goog.DEBUG && !COMPILED) {
  /**
   * @public
   * @override
   */
  proto.aptos.transaction.testing1.v1.WriteTableItem.displayName = 'proto.aptos.transaction.testing1.v1.WriteTableItem';
}
/**
 * Generated by JsPbCodeGenerator.
 * @param {Array=} opt_data Optional initial data array, typically from a
 * server response, or constructed directly in Javascript. The array is used
 * in place and becomes part of the constructed object. It is not cloned.
 * If no data is provided, the constructed object will be empty, but still
 * valid.
 * @extends {jspb.Message}
 * @constructor
 */
proto.aptos.transaction.testing1.v1.TransactionPayload = function(opt_data) {
  jspb.Message.initialize(this, opt_data, 0, -1, null, proto.aptos.transaction.testing1.v1.TransactionPayload.oneofGroups_);
};
goog.inherits(proto.aptos.transaction.testing1.v1.TransactionPayload, jspb.Message);
if (goog.DEBUG && !COMPILED) {
  /**
   * @public
   * @override
   */
  proto.aptos.transaction.testing1.v1.TransactionPayload.displayName = 'proto.aptos.transaction.testing1.v1.TransactionPayload';
}
/**
 * Generated by JsPbCodeGenerator.
 * @param {Array=} opt_data Optional initial data array, typically from a
 * server response, or constructed directly in Javascript. The array is used
 * in place and becomes part of the constructed object. It is not cloned.
 * If no data is provided, the constructed object will be empty, but still
 * valid.
 * @extends {jspb.Message}
 * @constructor
 */
proto.aptos.transaction.testing1.v1.EntryFunctionPayload = function(opt_data) {
  jspb.Message.initialize(this, opt_data, 0, -1, proto.aptos.transaction.testing1.v1.EntryFunctionPayload.repeatedFields_, null);
};
goog.inherits(proto.aptos.transaction.testing1.v1.EntryFunctionPayload, jspb.Message);
if (goog.DEBUG && !COMPILED) {
  /**
   * @public
   * @override
   */
  proto.aptos.transaction.testing1.v1.EntryFunctionPayload.displayName = 'proto.aptos.transaction.testing1.v1.EntryFunctionPayload';
}
/**
 * Generated by JsPbCodeGenerator.
 * @param {Array=} opt_data Optional initial data array, typically from a
 * server response, or constructed directly in Javascript. The array is used
 * in place and becomes part of the constructed object. It is not cloned.
 * If no data is provided, the constructed object will be empty, but still
 * valid.
 * @extends {jspb.Message}
 * @constructor
 */
proto.aptos.transaction.testing1.v1.MoveScriptBytecode = function(opt_data) {
  jspb.Message.initialize(this, opt_data, 0, -1, null, null);
};
goog.inherits(proto.aptos.transaction.testing1.v1.MoveScriptBytecode, jspb.Message);
if (goog.DEBUG && !COMPILED) {
  /**
   * @public
   * @override
   */
  proto.aptos.transaction.testing1.v1.MoveScriptBytecode.displayName = 'proto.aptos.transaction.testing1.v1.MoveScriptBytecode';
}
/**
 * Generated by JsPbCodeGenerator.
 * @param {Array=} opt_data Optional initial data array, typically from a
 * server response, or constructed directly in Javascript. The array is used
 * in place and becomes part of the constructed object. It is not cloned.
 * If no data is provided, the constructed object will be empty, but still
 * valid.
 * @extends {jspb.Message}
 * @constructor
 */
proto.aptos.transaction.testing1.v1.ScriptPayload = function(opt_data) {
  jspb.Message.initialize(this, opt_data, 0, -1, proto.aptos.transaction.testing1.v1.ScriptPayload.repeatedFields_, null);
};
goog.inherits(proto.aptos.transaction.testing1.v1.ScriptPayload, jspb.Message);
if (goog.DEBUG && !COMPILED) {
  /**
   * @public
   * @override
   */
  proto.aptos.transaction.testing1.v1.ScriptPayload.displayName = 'proto.aptos.transaction.testing1.v1.ScriptPayload';
}
/**
 * Generated by JsPbCodeGenerator.
 * @param {Array=} opt_data Optional initial data array, typically from a
 * server response, or constructed directly in Javascript. The array is used
 * in place and becomes part of the constructed object. It is not cloned.
 * If no data is provided, the constructed object will be empty, but still
 * valid.
 * @extends {jspb.Message}
 * @constructor
 */
proto.aptos.transaction.testing1.v1.MultisigPayload = function(opt_data) {
  jspb.Message.initialize(this, opt_data, 0, -1, null, null);
};
goog.inherits(proto.aptos.transaction.testing1.v1.MultisigPayload, jspb.Message);
if (goog.DEBUG && !COMPILED) {
  /**
   * @public
   * @override
   */
  proto.aptos.transaction.testing1.v1.MultisigPayload.displayName = 'proto.aptos.transaction.testing1.v1.MultisigPayload';
}
/**
 * Generated by JsPbCodeGenerator.
 * @param {Array=} opt_data Optional initial data array, typically from a
 * server response, or constructed directly in Javascript. The array is used
 * in place and becomes part of the constructed object. It is not cloned.
 * If no data is provided, the constructed object will be empty, but still
 * valid.
 * @extends {jspb.Message}
 * @constructor
 */
proto.aptos.transaction.testing1.v1.MultisigTransactionPayload = function(opt_data) {
  jspb.Message.initialize(this, opt_data, 0, -1, null, proto.aptos.transaction.testing1.v1.MultisigTransactionPayload.oneofGroups_);
};
goog.inherits(proto.aptos.transaction.testing1.v1.MultisigTransactionPayload, jspb.Message);
if (goog.DEBUG && !COMPILED) {
  /**
   * @public
   * @override
   */
  proto.aptos.transaction.testing1.v1.MultisigTransactionPayload.displayName = 'proto.aptos.transaction.testing1.v1.MultisigTransactionPayload';
}
/**
 * Generated by JsPbCodeGenerator.
 * @param {Array=} opt_data Optional initial data array, typically from a
 * server response, or constructed directly in Javascript. The array is used
 * in place and becomes part of the constructed object. It is not cloned.
 * If no data is provided, the constructed object will be empty, but still
 * valid.
 * @extends {jspb.Message}
 * @constructor
 */
proto.aptos.transaction.testing1.v1.ModuleBundlePayload = function(opt_data) {
  jspb.Message.initialize(this, opt_data, 0, -1, proto.aptos.transaction.testing1.v1.ModuleBundlePayload.repeatedFields_, null);
};
goog.inherits(proto.aptos.transaction.testing1.v1.ModuleBundlePayload, jspb.Message);
if (goog.DEBUG && !COMPILED) {
  /**
   * @public
   * @override
   */
  proto.aptos.transaction.testing1.v1.ModuleBundlePayload.displayName = 'proto.aptos.transaction.testing1.v1.ModuleBundlePayload';
}
/**
 * Generated by JsPbCodeGenerator.
 * @param {Array=} opt_data Optional initial data array, typically from a
 * server response, or constructed directly in Javascript. The array is used
 * in place and becomes part of the constructed object. It is not cloned.
 * If no data is provided, the constructed object will be empty, but still
 * valid.
 * @extends {jspb.Message}
 * @constructor
 */
proto.aptos.transaction.testing1.v1.MoveModuleBytecode = function(opt_data) {
  jspb.Message.initialize(this, opt_data, 0, -1, null, null);
};
goog.inherits(proto.aptos.transaction.testing1.v1.MoveModuleBytecode, jspb.Message);
if (goog.DEBUG && !COMPILED) {
  /**
   * @public
   * @override
   */
  proto.aptos.transaction.testing1.v1.MoveModuleBytecode.displayName = 'proto.aptos.transaction.testing1.v1.MoveModuleBytecode';
}
/**
 * Generated by JsPbCodeGenerator.
 * @param {Array=} opt_data Optional initial data array, typically from a
 * server response, or constructed directly in Javascript. The array is used
 * in place and becomes part of the constructed object. It is not cloned.
 * If no data is provided, the constructed object will be empty, but still
 * valid.
 * @extends {jspb.Message}
 * @constructor
 */
proto.aptos.transaction.testing1.v1.MoveModule = function(opt_data) {
  jspb.Message.initialize(this, opt_data, 0, -1, proto.aptos.transaction.testing1.v1.MoveModule.repeatedFields_, null);
};
goog.inherits(proto.aptos.transaction.testing1.v1.MoveModule, jspb.Message);
if (goog.DEBUG && !COMPILED) {
  /**
   * @public
   * @override
   */
  proto.aptos.transaction.testing1.v1.MoveModule.displayName = 'proto.aptos.transaction.testing1.v1.MoveModule';
}
/**
 * Generated by JsPbCodeGenerator.
 * @param {Array=} opt_data Optional initial data array, typically from a
 * server response, or constructed directly in Javascript. The array is used
 * in place and becomes part of the constructed object. It is not cloned.
 * If no data is provided, the constructed object will be empty, but still
 * valid.
 * @extends {jspb.Message}
 * @constructor
 */
proto.aptos.transaction.testing1.v1.MoveFunction = function(opt_data) {
  jspb.Message.initialize(this, opt_data, 0, -1, proto.aptos.transaction.testing1.v1.MoveFunction.repeatedFields_, null);
};
goog.inherits(proto.aptos.transaction.testing1.v1.MoveFunction, jspb.Message);
if (goog.DEBUG && !COMPILED) {
  /**
   * @public
   * @override
   */
  proto.aptos.transaction.testing1.v1.MoveFunction.displayName = 'proto.aptos.transaction.testing1.v1.MoveFunction';
}
/**
 * Generated by JsPbCodeGenerator.
 * @param {Array=} opt_data Optional initial data array, typically from a
 * server response, or constructed directly in Javascript. The array is used
 * in place and becomes part of the constructed object. It is not cloned.
 * If no data is provided, the constructed object will be empty, but still
 * valid.
 * @extends {jspb.Message}
 * @constructor
 */
proto.aptos.transaction.testing1.v1.MoveStruct = function(opt_data) {
  jspb.Message.initialize(this, opt_data, 0, -1, proto.aptos.transaction.testing1.v1.MoveStruct.repeatedFields_, null);
};
goog.inherits(proto.aptos.transaction.testing1.v1.MoveStruct, jspb.Message);
if (goog.DEBUG && !COMPILED) {
  /**
   * @public
   * @override
   */
  proto.aptos.transaction.testing1.v1.MoveStruct.displayName = 'proto.aptos.transaction.testing1.v1.MoveStruct';
}
/**
 * Generated by JsPbCodeGenerator.
 * @param {Array=} opt_data Optional initial data array, typically from a
 * server response, or constructed directly in Javascript. The array is used
 * in place and becomes part of the constructed object. It is not cloned.
 * If no data is provided, the constructed object will be empty, but still
 * valid.
 * @extends {jspb.Message}
 * @constructor
 */
proto.aptos.transaction.testing1.v1.MoveStructGenericTypeParam = function(opt_data) {
  jspb.Message.initialize(this, opt_data, 0, -1, proto.aptos.transaction.testing1.v1.MoveStructGenericTypeParam.repeatedFields_, null);
};
goog.inherits(proto.aptos.transaction.testing1.v1.MoveStructGenericTypeParam, jspb.Message);
if (goog.DEBUG && !COMPILED) {
  /**
   * @public
   * @override
   */
  proto.aptos.transaction.testing1.v1.MoveStructGenericTypeParam.displayName = 'proto.aptos.transaction.testing1.v1.MoveStructGenericTypeParam';
}
/**
 * Generated by JsPbCodeGenerator.
 * @param {Array=} opt_data Optional initial data array, typically from a
 * server response, or constructed directly in Javascript. The array is used
 * in place and becomes part of the constructed object. It is not cloned.
 * If no data is provided, the constructed object will be empty, but still
 * valid.
 * @extends {jspb.Message}
 * @constructor
 */
proto.aptos.transaction.testing1.v1.MoveStructField = function(opt_data) {
  jspb.Message.initialize(this, opt_data, 0, -1, null, null);
};
goog.inherits(proto.aptos.transaction.testing1.v1.MoveStructField, jspb.Message);
if (goog.DEBUG && !COMPILED) {
  /**
   * @public
   * @override
   */
  proto.aptos.transaction.testing1.v1.MoveStructField.displayName = 'proto.aptos.transaction.testing1.v1.MoveStructField';
}
/**
 * Generated by JsPbCodeGenerator.
 * @param {Array=} opt_data Optional initial data array, typically from a
 * server response, or constructed directly in Javascript. The array is used
 * in place and becomes part of the constructed object. It is not cloned.
 * If no data is provided, the constructed object will be empty, but still
 * valid.
 * @extends {jspb.Message}
 * @constructor
 */
proto.aptos.transaction.testing1.v1.MoveFunctionGenericTypeParam = function(opt_data) {
  jspb.Message.initialize(this, opt_data, 0, -1, proto.aptos.transaction.testing1.v1.MoveFunctionGenericTypeParam.repeatedFields_, null);
};
goog.inherits(proto.aptos.transaction.testing1.v1.MoveFunctionGenericTypeParam, jspb.Message);
if (goog.DEBUG && !COMPILED) {
  /**
   * @public
   * @override
   */
  proto.aptos.transaction.testing1.v1.MoveFunctionGenericTypeParam.displayName = 'proto.aptos.transaction.testing1.v1.MoveFunctionGenericTypeParam';
}
/**
 * Generated by JsPbCodeGenerator.
 * @param {Array=} opt_data Optional initial data array, typically from a
 * server response, or constructed directly in Javascript. The array is used
 * in place and becomes part of the constructed object. It is not cloned.
 * If no data is provided, the constructed object will be empty, but still
 * valid.
 * @extends {jspb.Message}
 * @constructor
 */
proto.aptos.transaction.testing1.v1.MoveType = function(opt_data) {
  jspb.Message.initialize(this, opt_data, 0, -1, null, proto.aptos.transaction.testing1.v1.MoveType.oneofGroups_);
};
goog.inherits(proto.aptos.transaction.testing1.v1.MoveType, jspb.Message);
if (goog.DEBUG && !COMPILED) {
  /**
   * @public
   * @override
   */
  proto.aptos.transaction.testing1.v1.MoveType.displayName = 'proto.aptos.transaction.testing1.v1.MoveType';
}
/**
 * Generated by JsPbCodeGenerator.
 * @param {Array=} opt_data Optional initial data array, typically from a
 * server response, or constructed directly in Javascript. The array is used
 * in place and becomes part of the constructed object. It is not cloned.
 * If no data is provided, the constructed object will be empty, but still
 * valid.
 * @extends {jspb.Message}
 * @constructor
 */
proto.aptos.transaction.testing1.v1.MoveType.ReferenceType = function(opt_data) {
  jspb.Message.initialize(this, opt_data, 0, -1, null, null);
};
goog.inherits(proto.aptos.transaction.testing1.v1.MoveType.ReferenceType, jspb.Message);
if (goog.DEBUG && !COMPILED) {
  /**
   * @public
   * @override
   */
  proto.aptos.transaction.testing1.v1.MoveType.ReferenceType.displayName = 'proto.aptos.transaction.testing1.v1.MoveType.ReferenceType';
}
/**
 * Generated by JsPbCodeGenerator.
 * @param {Array=} opt_data Optional initial data array, typically from a
 * server response, or constructed directly in Javascript. The array is used
 * in place and becomes part of the constructed object. It is not cloned.
 * If no data is provided, the constructed object will be empty, but still
 * valid.
 * @extends {jspb.Message}
 * @constructor
 */
proto.aptos.transaction.testing1.v1.WriteSetPayload = function(opt_data) {
  jspb.Message.initialize(this, opt_data, 0, -1, null, null);
};
goog.inherits(proto.aptos.transaction.testing1.v1.WriteSetPayload, jspb.Message);
if (goog.DEBUG && !COMPILED) {
  /**
   * @public
   * @override
   */
  proto.aptos.transaction.testing1.v1.WriteSetPayload.displayName = 'proto.aptos.transaction.testing1.v1.WriteSetPayload';
}
/**
 * Generated by JsPbCodeGenerator.
 * @param {Array=} opt_data Optional initial data array, typically from a
 * server response, or constructed directly in Javascript. The array is used
 * in place and becomes part of the constructed object. It is not cloned.
 * If no data is provided, the constructed object will be empty, but still
 * valid.
 * @extends {jspb.Message}
 * @constructor
 */
proto.aptos.transaction.testing1.v1.EntryFunctionId = function(opt_data) {
  jspb.Message.initialize(this, opt_data, 0, -1, null, null);
};
goog.inherits(proto.aptos.transaction.testing1.v1.EntryFunctionId, jspb.Message);
if (goog.DEBUG && !COMPILED) {
  /**
   * @public
   * @override
   */
  proto.aptos.transaction.testing1.v1.EntryFunctionId.displayName = 'proto.aptos.transaction.testing1.v1.EntryFunctionId';
}
/**
 * Generated by JsPbCodeGenerator.
 * @param {Array=} opt_data Optional initial data array, typically from a
 * server response, or constructed directly in Javascript. The array is used
 * in place and becomes part of the constructed object. It is not cloned.
 * If no data is provided, the constructed object will be empty, but still
 * valid.
 * @extends {jspb.Message}
 * @constructor
 */
proto.aptos.transaction.testing1.v1.MoveModuleId = function(opt_data) {
  jspb.Message.initialize(this, opt_data, 0, -1, null, null);
};
goog.inherits(proto.aptos.transaction.testing1.v1.MoveModuleId, jspb.Message);
if (goog.DEBUG && !COMPILED) {
  /**
   * @public
   * @override
   */
  proto.aptos.transaction.testing1.v1.MoveModuleId.displayName = 'proto.aptos.transaction.testing1.v1.MoveModuleId';
}
/**
 * Generated by JsPbCodeGenerator.
 * @param {Array=} opt_data Optional initial data array, typically from a
 * server response, or constructed directly in Javascript. The array is used
 * in place and becomes part of the constructed object. It is not cloned.
 * If no data is provided, the constructed object will be empty, but still
 * valid.
 * @extends {jspb.Message}
 * @constructor
 */
proto.aptos.transaction.testing1.v1.MoveStructTag = function(opt_data) {
  jspb.Message.initialize(this, opt_data, 0, -1, proto.aptos.transaction.testing1.v1.MoveStructTag.repeatedFields_, null);
};
goog.inherits(proto.aptos.transaction.testing1.v1.MoveStructTag, jspb.Message);
if (goog.DEBUG && !COMPILED) {
  /**
   * @public
   * @override
   */
  proto.aptos.transaction.testing1.v1.MoveStructTag.displayName = 'proto.aptos.transaction.testing1.v1.MoveStructTag';
}
/**
 * Generated by JsPbCodeGenerator.
 * @param {Array=} opt_data Optional initial data array, typically from a
 * server response, or constructed directly in Javascript. The array is used
 * in place and becomes part of the constructed object. It is not cloned.
 * If no data is provided, the constructed object will be empty, but still
 * valid.
 * @extends {jspb.Message}
 * @constructor
 */
proto.aptos.transaction.testing1.v1.Signature = function(opt_data) {
  jspb.Message.initialize(this, opt_data, 0, -1, null, proto.aptos.transaction.testing1.v1.Signature.oneofGroups_);
};
goog.inherits(proto.aptos.transaction.testing1.v1.Signature, jspb.Message);
if (goog.DEBUG && !COMPILED) {
  /**
   * @public
   * @override
   */
  proto.aptos.transaction.testing1.v1.Signature.displayName = 'proto.aptos.transaction.testing1.v1.Signature';
}
/**
 * Generated by JsPbCodeGenerator.
 * @param {Array=} opt_data Optional initial data array, typically from a
 * server response, or constructed directly in Javascript. The array is used
 * in place and becomes part of the constructed object. It is not cloned.
 * If no data is provided, the constructed object will be empty, but still
 * valid.
 * @extends {jspb.Message}
 * @constructor
 */
proto.aptos.transaction.testing1.v1.Ed25519Signature = function(opt_data) {
  jspb.Message.initialize(this, opt_data, 0, -1, null, null);
};
goog.inherits(proto.aptos.transaction.testing1.v1.Ed25519Signature, jspb.Message);
if (goog.DEBUG && !COMPILED) {
  /**
   * @public
   * @override
   */
  proto.aptos.transaction.testing1.v1.Ed25519Signature.displayName = 'proto.aptos.transaction.testing1.v1.Ed25519Signature';
}
/**
 * Generated by JsPbCodeGenerator.
 * @param {Array=} opt_data Optional initial data array, typically from a
 * server response, or constructed directly in Javascript. The array is used
 * in place and becomes part of the constructed object. It is not cloned.
 * If no data is provided, the constructed object will be empty, but still
 * valid.
 * @extends {jspb.Message}
 * @constructor
 */
proto.aptos.transaction.testing1.v1.MultiEd25519Signature = function(opt_data) {
  jspb.Message.initialize(this, opt_data, 0, -1, proto.aptos.transaction.testing1.v1.MultiEd25519Signature.repeatedFields_, null);
};
goog.inherits(proto.aptos.transaction.testing1.v1.MultiEd25519Signature, jspb.Message);
if (goog.DEBUG && !COMPILED) {
  /**
   * @public
   * @override
   */
  proto.aptos.transaction.testing1.v1.MultiEd25519Signature.displayName = 'proto.aptos.transaction.testing1.v1.MultiEd25519Signature';
}
/**
 * Generated by JsPbCodeGenerator.
 * @param {Array=} opt_data Optional initial data array, typically from a
 * server response, or constructed directly in Javascript. The array is used
 * in place and becomes part of the constructed object. It is not cloned.
 * If no data is provided, the constructed object will be empty, but still
 * valid.
 * @extends {jspb.Message}
 * @constructor
 */
proto.aptos.transaction.testing1.v1.MultiAgentSignature = function(opt_data) {
  jspb.Message.initialize(this, opt_data, 0, -1, proto.aptos.transaction.testing1.v1.MultiAgentSignature.repeatedFields_, null);
};
goog.inherits(proto.aptos.transaction.testing1.v1.MultiAgentSignature, jspb.Message);
if (goog.DEBUG && !COMPILED) {
  /**
   * @public
   * @override
   */
  proto.aptos.transaction.testing1.v1.MultiAgentSignature.displayName = 'proto.aptos.transaction.testing1.v1.MultiAgentSignature';
}
/**
 * Generated by JsPbCodeGenerator.
 * @param {Array=} opt_data Optional initial data array, typically from a
 * server response, or constructed directly in Javascript. The array is used
 * in place and becomes part of the constructed object. It is not cloned.
 * If no data is provided, the constructed object will be empty, but still
 * valid.
 * @extends {jspb.Message}
 * @constructor
 */
proto.aptos.transaction.testing1.v1.AccountSignature = function(opt_data) {
  jspb.Message.initialize(this, opt_data, 0, -1, null, proto.aptos.transaction.testing1.v1.AccountSignature.oneofGroups_);
};
goog.inherits(proto.aptos.transaction.testing1.v1.AccountSignature, jspb.Message);
if (goog.DEBUG && !COMPILED) {
  /**
   * @public
   * @override
   */
  proto.aptos.transaction.testing1.v1.AccountSignature.displayName = 'proto.aptos.transaction.testing1.v1.AccountSignature';
}

/**
 * List of repeated fields within this message type.
 * @private {!Array<number>}
 * @const
 */
proto.aptos.transaction.testing1.v1.Block.repeatedFields_ = [3];



if (jspb.Message.GENERATE_TO_OBJECT) {
/**
 * Creates an object representation of this proto.
 * Field names that are reserved in JavaScript and will be renamed to pb_name.
 * Optional fields that are not set will be set to undefined.
 * To access a reserved field use, foo.pb_<name>, eg, foo.pb_default.
 * For the list of reserved names please see:
 *     net/proto2/compiler/js/internal/generator.cc#kKeyword.
 * @param {boolean=} opt_includeInstance Deprecated. whether to include the
 *     JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @return {!Object}
 */
proto.aptos.transaction.testing1.v1.Block.prototype.toObject = function(opt_includeInstance) {
  return proto.aptos.transaction.testing1.v1.Block.toObject(opt_includeInstance, this);
};


/**
 * Static version of the {@see toObject} method.
 * @param {boolean|undefined} includeInstance Deprecated. Whether to include
 *     the JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @param {!proto.aptos.transaction.testing1.v1.Block} msg The msg instance to transform.
 * @return {!Object}
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.aptos.transaction.testing1.v1.Block.toObject = function(includeInstance, msg) {
  var f, obj = {
    timestamp: (f = msg.getTimestamp()) && aptos_util_timestamp_timestamp_pb.Timestamp.toObject(includeInstance, f),
    height: jspb.Message.getFieldWithDefault(msg, 2, 0),
    transactionsList: jspb.Message.toObjectList(msg.getTransactionsList(),
    proto.aptos.transaction.testing1.v1.Transaction.toObject, includeInstance),
    chainId: jspb.Message.getFieldWithDefault(msg, 4, 0)
  };

  if (includeInstance) {
    obj.$jspbMessageInstance = msg;
  }
  return obj;
};
}


/**
 * Deserializes binary data (in protobuf wire format).
 * @param {jspb.ByteSource} bytes The bytes to deserialize.
 * @return {!proto.aptos.transaction.testing1.v1.Block}
 */
proto.aptos.transaction.testing1.v1.Block.deserializeBinary = function(bytes) {
  var reader = new jspb.BinaryReader(bytes);
  var msg = new proto.aptos.transaction.testing1.v1.Block;
  return proto.aptos.transaction.testing1.v1.Block.deserializeBinaryFromReader(msg, reader);
};


/**
 * Deserializes binary data (in protobuf wire format) from the
 * given reader into the given message object.
 * @param {!proto.aptos.transaction.testing1.v1.Block} msg The message object to deserialize into.
 * @param {!jspb.BinaryReader} reader The BinaryReader to use.
 * @return {!proto.aptos.transaction.testing1.v1.Block}
 */
proto.aptos.transaction.testing1.v1.Block.deserializeBinaryFromReader = function(msg, reader) {
  while (reader.nextField()) {
    if (reader.isEndGroup()) {
      break;
    }
    var field = reader.getFieldNumber();
    switch (field) {
    case 1:
      var value = new aptos_util_timestamp_timestamp_pb.Timestamp;
      reader.readMessage(value,aptos_util_timestamp_timestamp_pb.Timestamp.deserializeBinaryFromReader);
      msg.setTimestamp(value);
      break;
    case 2:
      var value = /** @type {number} */ (reader.readUint64());
      msg.setHeight(value);
      break;
    case 3:
      var value = new proto.aptos.transaction.testing1.v1.Transaction;
      reader.readMessage(value,proto.aptos.transaction.testing1.v1.Transaction.deserializeBinaryFromReader);
      msg.addTransactions(value);
      break;
    case 4:
      var value = /** @type {number} */ (reader.readUint32());
      msg.setChainId(value);
      break;
    default:
      reader.skipField();
      break;
    }
  }
  return msg;
};


/**
 * Serializes the message to binary data (in protobuf wire format).
 * @return {!Uint8Array}
 */
proto.aptos.transaction.testing1.v1.Block.prototype.serializeBinary = function() {
  var writer = new jspb.BinaryWriter();
  proto.aptos.transaction.testing1.v1.Block.serializeBinaryToWriter(this, writer);
  return writer.getResultBuffer();
};


/**
 * Serializes the given message to binary data (in protobuf wire
 * format), writing to the given BinaryWriter.
 * @param {!proto.aptos.transaction.testing1.v1.Block} message
 * @param {!jspb.BinaryWriter} writer
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.aptos.transaction.testing1.v1.Block.serializeBinaryToWriter = function(message, writer) {
  var f = undefined;
  f = message.getTimestamp();
  if (f != null) {
    writer.writeMessage(
      1,
      f,
      aptos_util_timestamp_timestamp_pb.Timestamp.serializeBinaryToWriter
    );
  }
  f = message.getHeight();
  if (f !== 0) {
    writer.writeUint64(
      2,
      f
    );
  }
  f = message.getTransactionsList();
  if (f.length > 0) {
    writer.writeRepeatedMessage(
      3,
      f,
      proto.aptos.transaction.testing1.v1.Transaction.serializeBinaryToWriter
    );
  }
  f = message.getChainId();
  if (f !== 0) {
    writer.writeUint32(
      4,
      f
    );
  }
};


/**
 * optional aptos.util.timestamp.Timestamp timestamp = 1;
 * @return {?proto.aptos.util.timestamp.Timestamp}
 */
proto.aptos.transaction.testing1.v1.Block.prototype.getTimestamp = function() {
  return /** @type{?proto.aptos.util.timestamp.Timestamp} */ (
    jspb.Message.getWrapperField(this, aptos_util_timestamp_timestamp_pb.Timestamp, 1));
};


/**
 * @param {?proto.aptos.util.timestamp.Timestamp|undefined} value
 * @return {!proto.aptos.transaction.testing1.v1.Block} returns this
*/
proto.aptos.transaction.testing1.v1.Block.prototype.setTimestamp = function(value) {
  return jspb.Message.setWrapperField(this, 1, value);
};


/**
 * Clears the message field making it undefined.
 * @return {!proto.aptos.transaction.testing1.v1.Block} returns this
 */
proto.aptos.transaction.testing1.v1.Block.prototype.clearTimestamp = function() {
  return this.setTimestamp(undefined);
};


/**
 * Returns whether this field is set.
 * @return {boolean}
 */
proto.aptos.transaction.testing1.v1.Block.prototype.hasTimestamp = function() {
  return jspb.Message.getField(this, 1) != null;
};


/**
 * optional uint64 height = 2;
 * @return {number}
 */
proto.aptos.transaction.testing1.v1.Block.prototype.getHeight = function() {
  return /** @type {number} */ (jspb.Message.getFieldWithDefault(this, 2, 0));
};


/**
 * @param {number} value
 * @return {!proto.aptos.transaction.testing1.v1.Block} returns this
 */
proto.aptos.transaction.testing1.v1.Block.prototype.setHeight = function(value) {
  return jspb.Message.setProto3IntField(this, 2, value);
};


/**
 * repeated Transaction transactions = 3;
 * @return {!Array<!proto.aptos.transaction.testing1.v1.Transaction>}
 */
proto.aptos.transaction.testing1.v1.Block.prototype.getTransactionsList = function() {
  return /** @type{!Array<!proto.aptos.transaction.testing1.v1.Transaction>} */ (
    jspb.Message.getRepeatedWrapperField(this, proto.aptos.transaction.testing1.v1.Transaction, 3));
};


/**
 * @param {!Array<!proto.aptos.transaction.testing1.v1.Transaction>} value
 * @return {!proto.aptos.transaction.testing1.v1.Block} returns this
*/
proto.aptos.transaction.testing1.v1.Block.prototype.setTransactionsList = function(value) {
  return jspb.Message.setRepeatedWrapperField(this, 3, value);
};


/**
 * @param {!proto.aptos.transaction.testing1.v1.Transaction=} opt_value
 * @param {number=} opt_index
 * @return {!proto.aptos.transaction.testing1.v1.Transaction}
 */
proto.aptos.transaction.testing1.v1.Block.prototype.addTransactions = function(opt_value, opt_index) {
  return jspb.Message.addToRepeatedWrapperField(this, 3, opt_value, proto.aptos.transaction.testing1.v1.Transaction, opt_index);
};


/**
 * Clears the list making it empty but non-null.
 * @return {!proto.aptos.transaction.testing1.v1.Block} returns this
 */
proto.aptos.transaction.testing1.v1.Block.prototype.clearTransactionsList = function() {
  return this.setTransactionsList([]);
};


/**
 * optional uint32 chain_id = 4;
 * @return {number}
 */
proto.aptos.transaction.testing1.v1.Block.prototype.getChainId = function() {
  return /** @type {number} */ (jspb.Message.getFieldWithDefault(this, 4, 0));
};


/**
 * @param {number} value
 * @return {!proto.aptos.transaction.testing1.v1.Block} returns this
 */
proto.aptos.transaction.testing1.v1.Block.prototype.setChainId = function(value) {
  return jspb.Message.setProto3IntField(this, 4, value);
};



/**
 * Oneof group definitions for this message. Each group defines the field
 * numbers belonging to that group. When of these fields' value is set, all
 * other fields in the group are cleared. During deserialization, if multiple
 * fields are encountered for a group, only the last value seen will be kept.
 * @private {!Array<!Array<number>>}
 * @const
 */
proto.aptos.transaction.testing1.v1.Transaction.oneofGroups_ = [[7,8,9,10]];

/**
 * @enum {number}
 */
proto.aptos.transaction.testing1.v1.Transaction.TxnDataCase = {
  TXN_DATA_NOT_SET: 0,
  BLOCK_METADATA: 7,
  GENESIS: 8,
  STATE_CHECKPOINT: 9,
  USER: 10
};

/**
 * @return {proto.aptos.transaction.testing1.v1.Transaction.TxnDataCase}
 */
proto.aptos.transaction.testing1.v1.Transaction.prototype.getTxnDataCase = function() {
  return /** @type {proto.aptos.transaction.testing1.v1.Transaction.TxnDataCase} */(jspb.Message.computeOneofCase(this, proto.aptos.transaction.testing1.v1.Transaction.oneofGroups_[0]));
};



if (jspb.Message.GENERATE_TO_OBJECT) {
/**
 * Creates an object representation of this proto.
 * Field names that are reserved in JavaScript and will be renamed to pb_name.
 * Optional fields that are not set will be set to undefined.
 * To access a reserved field use, foo.pb_<name>, eg, foo.pb_default.
 * For the list of reserved names please see:
 *     net/proto2/compiler/js/internal/generator.cc#kKeyword.
 * @param {boolean=} opt_includeInstance Deprecated. whether to include the
 *     JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @return {!Object}
 */
proto.aptos.transaction.testing1.v1.Transaction.prototype.toObject = function(opt_includeInstance) {
  return proto.aptos.transaction.testing1.v1.Transaction.toObject(opt_includeInstance, this);
};


/**
 * Static version of the {@see toObject} method.
 * @param {boolean|undefined} includeInstance Deprecated. Whether to include
 *     the JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @param {!proto.aptos.transaction.testing1.v1.Transaction} msg The msg instance to transform.
 * @return {!Object}
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.aptos.transaction.testing1.v1.Transaction.toObject = function(includeInstance, msg) {
  var f, obj = {
    timestamp: (f = msg.getTimestamp()) && aptos_util_timestamp_timestamp_pb.Timestamp.toObject(includeInstance, f),
    version: jspb.Message.getFieldWithDefault(msg, 2, 0),
    info: (f = msg.getInfo()) && proto.aptos.transaction.testing1.v1.TransactionInfo.toObject(includeInstance, f),
    epoch: jspb.Message.getFieldWithDefault(msg, 4, 0),
    blockHeight: jspb.Message.getFieldWithDefault(msg, 5, 0),
    type: jspb.Message.getFieldWithDefault(msg, 6, 0),
    blockMetadata: (f = msg.getBlockMetadata()) && proto.aptos.transaction.testing1.v1.BlockMetadataTransaction.toObject(includeInstance, f),
    genesis: (f = msg.getGenesis()) && proto.aptos.transaction.testing1.v1.GenesisTransaction.toObject(includeInstance, f),
    stateCheckpoint: (f = msg.getStateCheckpoint()) && proto.aptos.transaction.testing1.v1.StateCheckpointTransaction.toObject(includeInstance, f),
    user: (f = msg.getUser()) && proto.aptos.transaction.testing1.v1.UserTransaction.toObject(includeInstance, f)
  };

  if (includeInstance) {
    obj.$jspbMessageInstance = msg;
  }
  return obj;
};
}


/**
 * Deserializes binary data (in protobuf wire format).
 * @param {jspb.ByteSource} bytes The bytes to deserialize.
 * @return {!proto.aptos.transaction.testing1.v1.Transaction}
 */
proto.aptos.transaction.testing1.v1.Transaction.deserializeBinary = function(bytes) {
  var reader = new jspb.BinaryReader(bytes);
  var msg = new proto.aptos.transaction.testing1.v1.Transaction;
  return proto.aptos.transaction.testing1.v1.Transaction.deserializeBinaryFromReader(msg, reader);
};


/**
 * Deserializes binary data (in protobuf wire format) from the
 * given reader into the given message object.
 * @param {!proto.aptos.transaction.testing1.v1.Transaction} msg The message object to deserialize into.
 * @param {!jspb.BinaryReader} reader The BinaryReader to use.
 * @return {!proto.aptos.transaction.testing1.v1.Transaction}
 */
proto.aptos.transaction.testing1.v1.Transaction.deserializeBinaryFromReader = function(msg, reader) {
  while (reader.nextField()) {
    if (reader.isEndGroup()) {
      break;
    }
    var field = reader.getFieldNumber();
    switch (field) {
    case 1:
      var value = new aptos_util_timestamp_timestamp_pb.Timestamp;
      reader.readMessage(value,aptos_util_timestamp_timestamp_pb.Timestamp.deserializeBinaryFromReader);
      msg.setTimestamp(value);
      break;
    case 2:
      var value = /** @type {number} */ (reader.readUint64());
      msg.setVersion(value);
      break;
    case 3:
      var value = new proto.aptos.transaction.testing1.v1.TransactionInfo;
      reader.readMessage(value,proto.aptos.transaction.testing1.v1.TransactionInfo.deserializeBinaryFromReader);
      msg.setInfo(value);
      break;
    case 4:
      var value = /** @type {number} */ (reader.readUint64());
      msg.setEpoch(value);
      break;
    case 5:
      var value = /** @type {number} */ (reader.readUint64());
      msg.setBlockHeight(value);
      break;
    case 6:
      var value = /** @type {!proto.aptos.transaction.testing1.v1.Transaction.TransactionType} */ (reader.readEnum());
      msg.setType(value);
      break;
    case 7:
      var value = new proto.aptos.transaction.testing1.v1.BlockMetadataTransaction;
      reader.readMessage(value,proto.aptos.transaction.testing1.v1.BlockMetadataTransaction.deserializeBinaryFromReader);
      msg.setBlockMetadata(value);
      break;
    case 8:
      var value = new proto.aptos.transaction.testing1.v1.GenesisTransaction;
      reader.readMessage(value,proto.aptos.transaction.testing1.v1.GenesisTransaction.deserializeBinaryFromReader);
      msg.setGenesis(value);
      break;
    case 9:
      var value = new proto.aptos.transaction.testing1.v1.StateCheckpointTransaction;
      reader.readMessage(value,proto.aptos.transaction.testing1.v1.StateCheckpointTransaction.deserializeBinaryFromReader);
      msg.setStateCheckpoint(value);
      break;
    case 10:
      var value = new proto.aptos.transaction.testing1.v1.UserTransaction;
      reader.readMessage(value,proto.aptos.transaction.testing1.v1.UserTransaction.deserializeBinaryFromReader);
      msg.setUser(value);
      break;
    default:
      reader.skipField();
      break;
    }
  }
  return msg;
};


/**
 * Serializes the message to binary data (in protobuf wire format).
 * @return {!Uint8Array}
 */
proto.aptos.transaction.testing1.v1.Transaction.prototype.serializeBinary = function() {
  var writer = new jspb.BinaryWriter();
  proto.aptos.transaction.testing1.v1.Transaction.serializeBinaryToWriter(this, writer);
  return writer.getResultBuffer();
};


/**
 * Serializes the given message to binary data (in protobuf wire
 * format), writing to the given BinaryWriter.
 * @param {!proto.aptos.transaction.testing1.v1.Transaction} message
 * @param {!jspb.BinaryWriter} writer
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.aptos.transaction.testing1.v1.Transaction.serializeBinaryToWriter = function(message, writer) {
  var f = undefined;
  f = message.getTimestamp();
  if (f != null) {
    writer.writeMessage(
      1,
      f,
      aptos_util_timestamp_timestamp_pb.Timestamp.serializeBinaryToWriter
    );
  }
  f = message.getVersion();
  if (f !== 0) {
    writer.writeUint64(
      2,
      f
    );
  }
  f = message.getInfo();
  if (f != null) {
    writer.writeMessage(
      3,
      f,
      proto.aptos.transaction.testing1.v1.TransactionInfo.serializeBinaryToWriter
    );
  }
  f = message.getEpoch();
  if (f !== 0) {
    writer.writeUint64(
      4,
      f
    );
  }
  f = message.getBlockHeight();
  if (f !== 0) {
    writer.writeUint64(
      5,
      f
    );
  }
  f = message.getType();
  if (f !== 0.0) {
    writer.writeEnum(
      6,
      f
    );
  }
  f = message.getBlockMetadata();
  if (f != null) {
    writer.writeMessage(
      7,
      f,
      proto.aptos.transaction.testing1.v1.BlockMetadataTransaction.serializeBinaryToWriter
    );
  }
  f = message.getGenesis();
  if (f != null) {
    writer.writeMessage(
      8,
      f,
      proto.aptos.transaction.testing1.v1.GenesisTransaction.serializeBinaryToWriter
    );
  }
  f = message.getStateCheckpoint();
  if (f != null) {
    writer.writeMessage(
      9,
      f,
      proto.aptos.transaction.testing1.v1.StateCheckpointTransaction.serializeBinaryToWriter
    );
  }
  f = message.getUser();
  if (f != null) {
    writer.writeMessage(
      10,
      f,
      proto.aptos.transaction.testing1.v1.UserTransaction.serializeBinaryToWriter
    );
  }
};


/**
 * @enum {number}
 */
proto.aptos.transaction.testing1.v1.Transaction.TransactionType = {
  TRANSACTION_TYPE_UNSPECIFIED: 0,
  TRANSACTION_TYPE_GENESIS: 1,
  TRANSACTION_TYPE_BLOCK_METADATA: 2,
  TRANSACTION_TYPE_STATE_CHECKPOINT: 3,
  TRANSACTION_TYPE_USER: 4
};

/**
 * optional aptos.util.timestamp.Timestamp timestamp = 1;
 * @return {?proto.aptos.util.timestamp.Timestamp}
 */
proto.aptos.transaction.testing1.v1.Transaction.prototype.getTimestamp = function() {
  return /** @type{?proto.aptos.util.timestamp.Timestamp} */ (
    jspb.Message.getWrapperField(this, aptos_util_timestamp_timestamp_pb.Timestamp, 1));
};


/**
 * @param {?proto.aptos.util.timestamp.Timestamp|undefined} value
 * @return {!proto.aptos.transaction.testing1.v1.Transaction} returns this
*/
proto.aptos.transaction.testing1.v1.Transaction.prototype.setTimestamp = function(value) {
  return jspb.Message.setWrapperField(this, 1, value);
};


/**
 * Clears the message field making it undefined.
 * @return {!proto.aptos.transaction.testing1.v1.Transaction} returns this
 */
proto.aptos.transaction.testing1.v1.Transaction.prototype.clearTimestamp = function() {
  return this.setTimestamp(undefined);
};


/**
 * Returns whether this field is set.
 * @return {boolean}
 */
proto.aptos.transaction.testing1.v1.Transaction.prototype.hasTimestamp = function() {
  return jspb.Message.getField(this, 1) != null;
};


/**
 * optional uint64 version = 2;
 * @return {number}
 */
proto.aptos.transaction.testing1.v1.Transaction.prototype.getVersion = function() {
  return /** @type {number} */ (jspb.Message.getFieldWithDefault(this, 2, 0));
};


/**
 * @param {number} value
 * @return {!proto.aptos.transaction.testing1.v1.Transaction} returns this
 */
proto.aptos.transaction.testing1.v1.Transaction.prototype.setVersion = function(value) {
  return jspb.Message.setProto3IntField(this, 2, value);
};


/**
 * optional TransactionInfo info = 3;
 * @return {?proto.aptos.transaction.testing1.v1.TransactionInfo}
 */
proto.aptos.transaction.testing1.v1.Transaction.prototype.getInfo = function() {
  return /** @type{?proto.aptos.transaction.testing1.v1.TransactionInfo} */ (
    jspb.Message.getWrapperField(this, proto.aptos.transaction.testing1.v1.TransactionInfo, 3));
};


/**
 * @param {?proto.aptos.transaction.testing1.v1.TransactionInfo|undefined} value
 * @return {!proto.aptos.transaction.testing1.v1.Transaction} returns this
*/
proto.aptos.transaction.testing1.v1.Transaction.prototype.setInfo = function(value) {
  return jspb.Message.setWrapperField(this, 3, value);
};


/**
 * Clears the message field making it undefined.
 * @return {!proto.aptos.transaction.testing1.v1.Transaction} returns this
 */
proto.aptos.transaction.testing1.v1.Transaction.prototype.clearInfo = function() {
  return this.setInfo(undefined);
};


/**
 * Returns whether this field is set.
 * @return {boolean}
 */
proto.aptos.transaction.testing1.v1.Transaction.prototype.hasInfo = function() {
  return jspb.Message.getField(this, 3) != null;
};


/**
 * optional uint64 epoch = 4;
 * @return {number}
 */
proto.aptos.transaction.testing1.v1.Transaction.prototype.getEpoch = function() {
  return /** @type {number} */ (jspb.Message.getFieldWithDefault(this, 4, 0));
};


/**
 * @param {number} value
 * @return {!proto.aptos.transaction.testing1.v1.Transaction} returns this
 */
proto.aptos.transaction.testing1.v1.Transaction.prototype.setEpoch = function(value) {
  return jspb.Message.setProto3IntField(this, 4, value);
};


/**
 * optional uint64 block_height = 5;
 * @return {number}
 */
proto.aptos.transaction.testing1.v1.Transaction.prototype.getBlockHeight = function() {
  return /** @type {number} */ (jspb.Message.getFieldWithDefault(this, 5, 0));
};


/**
 * @param {number} value
 * @return {!proto.aptos.transaction.testing1.v1.Transaction} returns this
 */
proto.aptos.transaction.testing1.v1.Transaction.prototype.setBlockHeight = function(value) {
  return jspb.Message.setProto3IntField(this, 5, value);
};


/**
 * optional TransactionType type = 6;
 * @return {!proto.aptos.transaction.testing1.v1.Transaction.TransactionType}
 */
proto.aptos.transaction.testing1.v1.Transaction.prototype.getType = function() {
  return /** @type {!proto.aptos.transaction.testing1.v1.Transaction.TransactionType} */ (jspb.Message.getFieldWithDefault(this, 6, 0));
};


/**
 * @param {!proto.aptos.transaction.testing1.v1.Transaction.TransactionType} value
 * @return {!proto.aptos.transaction.testing1.v1.Transaction} returns this
 */
proto.aptos.transaction.testing1.v1.Transaction.prototype.setType = function(value) {
  return jspb.Message.setProto3EnumField(this, 6, value);
};


/**
 * optional BlockMetadataTransaction block_metadata = 7;
 * @return {?proto.aptos.transaction.testing1.v1.BlockMetadataTransaction}
 */
proto.aptos.transaction.testing1.v1.Transaction.prototype.getBlockMetadata = function() {
  return /** @type{?proto.aptos.transaction.testing1.v1.BlockMetadataTransaction} */ (
    jspb.Message.getWrapperField(this, proto.aptos.transaction.testing1.v1.BlockMetadataTransaction, 7));
};


/**
 * @param {?proto.aptos.transaction.testing1.v1.BlockMetadataTransaction|undefined} value
 * @return {!proto.aptos.transaction.testing1.v1.Transaction} returns this
*/
proto.aptos.transaction.testing1.v1.Transaction.prototype.setBlockMetadata = function(value) {
  return jspb.Message.setOneofWrapperField(this, 7, proto.aptos.transaction.testing1.v1.Transaction.oneofGroups_[0], value);
};


/**
 * Clears the message field making it undefined.
 * @return {!proto.aptos.transaction.testing1.v1.Transaction} returns this
 */
proto.aptos.transaction.testing1.v1.Transaction.prototype.clearBlockMetadata = function() {
  return this.setBlockMetadata(undefined);
};


/**
 * Returns whether this field is set.
 * @return {boolean}
 */
proto.aptos.transaction.testing1.v1.Transaction.prototype.hasBlockMetadata = function() {
  return jspb.Message.getField(this, 7) != null;
};


/**
 * optional GenesisTransaction genesis = 8;
 * @return {?proto.aptos.transaction.testing1.v1.GenesisTransaction}
 */
proto.aptos.transaction.testing1.v1.Transaction.prototype.getGenesis = function() {
  return /** @type{?proto.aptos.transaction.testing1.v1.GenesisTransaction} */ (
    jspb.Message.getWrapperField(this, proto.aptos.transaction.testing1.v1.GenesisTransaction, 8));
};


/**
 * @param {?proto.aptos.transaction.testing1.v1.GenesisTransaction|undefined} value
 * @return {!proto.aptos.transaction.testing1.v1.Transaction} returns this
*/
proto.aptos.transaction.testing1.v1.Transaction.prototype.setGenesis = function(value) {
  return jspb.Message.setOneofWrapperField(this, 8, proto.aptos.transaction.testing1.v1.Transaction.oneofGroups_[0], value);
};


/**
 * Clears the message field making it undefined.
 * @return {!proto.aptos.transaction.testing1.v1.Transaction} returns this
 */
proto.aptos.transaction.testing1.v1.Transaction.prototype.clearGenesis = function() {
  return this.setGenesis(undefined);
};


/**
 * Returns whether this field is set.
 * @return {boolean}
 */
proto.aptos.transaction.testing1.v1.Transaction.prototype.hasGenesis = function() {
  return jspb.Message.getField(this, 8) != null;
};


/**
 * optional StateCheckpointTransaction state_checkpoint = 9;
 * @return {?proto.aptos.transaction.testing1.v1.StateCheckpointTransaction}
 */
proto.aptos.transaction.testing1.v1.Transaction.prototype.getStateCheckpoint = function() {
  return /** @type{?proto.aptos.transaction.testing1.v1.StateCheckpointTransaction} */ (
    jspb.Message.getWrapperField(this, proto.aptos.transaction.testing1.v1.StateCheckpointTransaction, 9));
};


/**
 * @param {?proto.aptos.transaction.testing1.v1.StateCheckpointTransaction|undefined} value
 * @return {!proto.aptos.transaction.testing1.v1.Transaction} returns this
*/
proto.aptos.transaction.testing1.v1.Transaction.prototype.setStateCheckpoint = function(value) {
  return jspb.Message.setOneofWrapperField(this, 9, proto.aptos.transaction.testing1.v1.Transaction.oneofGroups_[0], value);
};


/**
 * Clears the message field making it undefined.
 * @return {!proto.aptos.transaction.testing1.v1.Transaction} returns this
 */
proto.aptos.transaction.testing1.v1.Transaction.prototype.clearStateCheckpoint = function() {
  return this.setStateCheckpoint(undefined);
};


/**
 * Returns whether this field is set.
 * @return {boolean}
 */
proto.aptos.transaction.testing1.v1.Transaction.prototype.hasStateCheckpoint = function() {
  return jspb.Message.getField(this, 9) != null;
};


/**
 * optional UserTransaction user = 10;
 * @return {?proto.aptos.transaction.testing1.v1.UserTransaction}
 */
proto.aptos.transaction.testing1.v1.Transaction.prototype.getUser = function() {
  return /** @type{?proto.aptos.transaction.testing1.v1.UserTransaction} */ (
    jspb.Message.getWrapperField(this, proto.aptos.transaction.testing1.v1.UserTransaction, 10));
};


/**
 * @param {?proto.aptos.transaction.testing1.v1.UserTransaction|undefined} value
 * @return {!proto.aptos.transaction.testing1.v1.Transaction} returns this
*/
proto.aptos.transaction.testing1.v1.Transaction.prototype.setUser = function(value) {
  return jspb.Message.setOneofWrapperField(this, 10, proto.aptos.transaction.testing1.v1.Transaction.oneofGroups_[0], value);
};


/**
 * Clears the message field making it undefined.
 * @return {!proto.aptos.transaction.testing1.v1.Transaction} returns this
 */
proto.aptos.transaction.testing1.v1.Transaction.prototype.clearUser = function() {
  return this.setUser(undefined);
};


/**
 * Returns whether this field is set.
 * @return {boolean}
 */
proto.aptos.transaction.testing1.v1.Transaction.prototype.hasUser = function() {
  return jspb.Message.getField(this, 10) != null;
};



/**
 * List of repeated fields within this message type.
 * @private {!Array<number>}
 * @const
 */
proto.aptos.transaction.testing1.v1.BlockMetadataTransaction.repeatedFields_ = [3,6];



if (jspb.Message.GENERATE_TO_OBJECT) {
/**
 * Creates an object representation of this proto.
 * Field names that are reserved in JavaScript and will be renamed to pb_name.
 * Optional fields that are not set will be set to undefined.
 * To access a reserved field use, foo.pb_<name>, eg, foo.pb_default.
 * For the list of reserved names please see:
 *     net/proto2/compiler/js/internal/generator.cc#kKeyword.
 * @param {boolean=} opt_includeInstance Deprecated. whether to include the
 *     JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @return {!Object}
 */
proto.aptos.transaction.testing1.v1.BlockMetadataTransaction.prototype.toObject = function(opt_includeInstance) {
  return proto.aptos.transaction.testing1.v1.BlockMetadataTransaction.toObject(opt_includeInstance, this);
};


/**
 * Static version of the {@see toObject} method.
 * @param {boolean|undefined} includeInstance Deprecated. Whether to include
 *     the JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @param {!proto.aptos.transaction.testing1.v1.BlockMetadataTransaction} msg The msg instance to transform.
 * @return {!Object}
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.aptos.transaction.testing1.v1.BlockMetadataTransaction.toObject = function(includeInstance, msg) {
  var f, obj = {
    id: jspb.Message.getFieldWithDefault(msg, 1, ""),
    round: jspb.Message.getFieldWithDefault(msg, 2, 0),
    eventsList: jspb.Message.toObjectList(msg.getEventsList(),
    proto.aptos.transaction.testing1.v1.Event.toObject, includeInstance),
    previousBlockVotesBitvec: msg.getPreviousBlockVotesBitvec_asB64(),
    proposer: jspb.Message.getFieldWithDefault(msg, 5, ""),
    failedProposerIndicesList: (f = jspb.Message.getRepeatedField(msg, 6)) == null ? undefined : f
  };

  if (includeInstance) {
    obj.$jspbMessageInstance = msg;
  }
  return obj;
};
}


/**
 * Deserializes binary data (in protobuf wire format).
 * @param {jspb.ByteSource} bytes The bytes to deserialize.
 * @return {!proto.aptos.transaction.testing1.v1.BlockMetadataTransaction}
 */
proto.aptos.transaction.testing1.v1.BlockMetadataTransaction.deserializeBinary = function(bytes) {
  var reader = new jspb.BinaryReader(bytes);
  var msg = new proto.aptos.transaction.testing1.v1.BlockMetadataTransaction;
  return proto.aptos.transaction.testing1.v1.BlockMetadataTransaction.deserializeBinaryFromReader(msg, reader);
};


/**
 * Deserializes binary data (in protobuf wire format) from the
 * given reader into the given message object.
 * @param {!proto.aptos.transaction.testing1.v1.BlockMetadataTransaction} msg The message object to deserialize into.
 * @param {!jspb.BinaryReader} reader The BinaryReader to use.
 * @return {!proto.aptos.transaction.testing1.v1.BlockMetadataTransaction}
 */
proto.aptos.transaction.testing1.v1.BlockMetadataTransaction.deserializeBinaryFromReader = function(msg, reader) {
  while (reader.nextField()) {
    if (reader.isEndGroup()) {
      break;
    }
    var field = reader.getFieldNumber();
    switch (field) {
    case 1:
      var value = /** @type {string} */ (reader.readString());
      msg.setId(value);
      break;
    case 2:
      var value = /** @type {number} */ (reader.readUint64());
      msg.setRound(value);
      break;
    case 3:
      var value = new proto.aptos.transaction.testing1.v1.Event;
      reader.readMessage(value,proto.aptos.transaction.testing1.v1.Event.deserializeBinaryFromReader);
      msg.addEvents(value);
      break;
    case 4:
      var value = /** @type {!Uint8Array} */ (reader.readBytes());
      msg.setPreviousBlockVotesBitvec(value);
      break;
    case 5:
      var value = /** @type {string} */ (reader.readString());
      msg.setProposer(value);
      break;
    case 6:
      var values = /** @type {!Array<number>} */ (reader.isDelimited() ? reader.readPackedUint32() : [reader.readUint32()]);
      for (var i = 0; i < values.length; i++) {
        msg.addFailedProposerIndices(values[i]);
      }
      break;
    default:
      reader.skipField();
      break;
    }
  }
  return msg;
};


/**
 * Serializes the message to binary data (in protobuf wire format).
 * @return {!Uint8Array}
 */
proto.aptos.transaction.testing1.v1.BlockMetadataTransaction.prototype.serializeBinary = function() {
  var writer = new jspb.BinaryWriter();
  proto.aptos.transaction.testing1.v1.BlockMetadataTransaction.serializeBinaryToWriter(this, writer);
  return writer.getResultBuffer();
};


/**
 * Serializes the given message to binary data (in protobuf wire
 * format), writing to the given BinaryWriter.
 * @param {!proto.aptos.transaction.testing1.v1.BlockMetadataTransaction} message
 * @param {!jspb.BinaryWriter} writer
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.aptos.transaction.testing1.v1.BlockMetadataTransaction.serializeBinaryToWriter = function(message, writer) {
  var f = undefined;
  f = message.getId();
  if (f.length > 0) {
    writer.writeString(
      1,
      f
    );
  }
  f = message.getRound();
  if (f !== 0) {
    writer.writeUint64(
      2,
      f
    );
  }
  f = message.getEventsList();
  if (f.length > 0) {
    writer.writeRepeatedMessage(
      3,
      f,
      proto.aptos.transaction.testing1.v1.Event.serializeBinaryToWriter
    );
  }
  f = message.getPreviousBlockVotesBitvec_asU8();
  if (f.length > 0) {
    writer.writeBytes(
      4,
      f
    );
  }
  f = message.getProposer();
  if (f.length > 0) {
    writer.writeString(
      5,
      f
    );
  }
  f = message.getFailedProposerIndicesList();
  if (f.length > 0) {
    writer.writePackedUint32(
      6,
      f
    );
  }
};


/**
 * optional string id = 1;
 * @return {string}
 */
proto.aptos.transaction.testing1.v1.BlockMetadataTransaction.prototype.getId = function() {
  return /** @type {string} */ (jspb.Message.getFieldWithDefault(this, 1, ""));
};


/**
 * @param {string} value
 * @return {!proto.aptos.transaction.testing1.v1.BlockMetadataTransaction} returns this
 */
proto.aptos.transaction.testing1.v1.BlockMetadataTransaction.prototype.setId = function(value) {
  return jspb.Message.setProto3StringField(this, 1, value);
};


/**
 * optional uint64 round = 2;
 * @return {number}
 */
proto.aptos.transaction.testing1.v1.BlockMetadataTransaction.prototype.getRound = function() {
  return /** @type {number} */ (jspb.Message.getFieldWithDefault(this, 2, 0));
};


/**
 * @param {number} value
 * @return {!proto.aptos.transaction.testing1.v1.BlockMetadataTransaction} returns this
 */
proto.aptos.transaction.testing1.v1.BlockMetadataTransaction.prototype.setRound = function(value) {
  return jspb.Message.setProto3IntField(this, 2, value);
};


/**
 * repeated Event events = 3;
 * @return {!Array<!proto.aptos.transaction.testing1.v1.Event>}
 */
proto.aptos.transaction.testing1.v1.BlockMetadataTransaction.prototype.getEventsList = function() {
  return /** @type{!Array<!proto.aptos.transaction.testing1.v1.Event>} */ (
    jspb.Message.getRepeatedWrapperField(this, proto.aptos.transaction.testing1.v1.Event, 3));
};


/**
 * @param {!Array<!proto.aptos.transaction.testing1.v1.Event>} value
 * @return {!proto.aptos.transaction.testing1.v1.BlockMetadataTransaction} returns this
*/
proto.aptos.transaction.testing1.v1.BlockMetadataTransaction.prototype.setEventsList = function(value) {
  return jspb.Message.setRepeatedWrapperField(this, 3, value);
};


/**
 * @param {!proto.aptos.transaction.testing1.v1.Event=} opt_value
 * @param {number=} opt_index
 * @return {!proto.aptos.transaction.testing1.v1.Event}
 */
proto.aptos.transaction.testing1.v1.BlockMetadataTransaction.prototype.addEvents = function(opt_value, opt_index) {
  return jspb.Message.addToRepeatedWrapperField(this, 3, opt_value, proto.aptos.transaction.testing1.v1.Event, opt_index);
};


/**
 * Clears the list making it empty but non-null.
 * @return {!proto.aptos.transaction.testing1.v1.BlockMetadataTransaction} returns this
 */
proto.aptos.transaction.testing1.v1.BlockMetadataTransaction.prototype.clearEventsList = function() {
  return this.setEventsList([]);
};


/**
 * optional bytes previous_block_votes_bitvec = 4;
 * @return {!(string|Uint8Array)}
 */
proto.aptos.transaction.testing1.v1.BlockMetadataTransaction.prototype.getPreviousBlockVotesBitvec = function() {
  return /** @type {!(string|Uint8Array)} */ (jspb.Message.getFieldWithDefault(this, 4, ""));
};


/**
 * optional bytes previous_block_votes_bitvec = 4;
 * This is a type-conversion wrapper around `getPreviousBlockVotesBitvec()`
 * @return {string}
 */
proto.aptos.transaction.testing1.v1.BlockMetadataTransaction.prototype.getPreviousBlockVotesBitvec_asB64 = function() {
  return /** @type {string} */ (jspb.Message.bytesAsB64(
      this.getPreviousBlockVotesBitvec()));
};


/**
 * optional bytes previous_block_votes_bitvec = 4;
 * Note that Uint8Array is not supported on all browsers.
 * @see http://caniuse.com/Uint8Array
 * This is a type-conversion wrapper around `getPreviousBlockVotesBitvec()`
 * @return {!Uint8Array}
 */
proto.aptos.transaction.testing1.v1.BlockMetadataTransaction.prototype.getPreviousBlockVotesBitvec_asU8 = function() {
  return /** @type {!Uint8Array} */ (jspb.Message.bytesAsU8(
      this.getPreviousBlockVotesBitvec()));
};


/**
 * @param {!(string|Uint8Array)} value
 * @return {!proto.aptos.transaction.testing1.v1.BlockMetadataTransaction} returns this
 */
proto.aptos.transaction.testing1.v1.BlockMetadataTransaction.prototype.setPreviousBlockVotesBitvec = function(value) {
  return jspb.Message.setProto3BytesField(this, 4, value);
};


/**
 * optional string proposer = 5;
 * @return {string}
 */
proto.aptos.transaction.testing1.v1.BlockMetadataTransaction.prototype.getProposer = function() {
  return /** @type {string} */ (jspb.Message.getFieldWithDefault(this, 5, ""));
};


/**
 * @param {string} value
 * @return {!proto.aptos.transaction.testing1.v1.BlockMetadataTransaction} returns this
 */
proto.aptos.transaction.testing1.v1.BlockMetadataTransaction.prototype.setProposer = function(value) {
  return jspb.Message.setProto3StringField(this, 5, value);
};


/**
 * repeated uint32 failed_proposer_indices = 6;
 * @return {!Array<number>}
 */
proto.aptos.transaction.testing1.v1.BlockMetadataTransaction.prototype.getFailedProposerIndicesList = function() {
  return /** @type {!Array<number>} */ (jspb.Message.getRepeatedField(this, 6));
};


/**
 * @param {!Array<number>} value
 * @return {!proto.aptos.transaction.testing1.v1.BlockMetadataTransaction} returns this
 */
proto.aptos.transaction.testing1.v1.BlockMetadataTransaction.prototype.setFailedProposerIndicesList = function(value) {
  return jspb.Message.setField(this, 6, value || []);
};


/**
 * @param {number} value
 * @param {number=} opt_index
 * @return {!proto.aptos.transaction.testing1.v1.BlockMetadataTransaction} returns this
 */
proto.aptos.transaction.testing1.v1.BlockMetadataTransaction.prototype.addFailedProposerIndices = function(value, opt_index) {
  return jspb.Message.addToRepeatedField(this, 6, value, opt_index);
};


/**
 * Clears the list making it empty but non-null.
 * @return {!proto.aptos.transaction.testing1.v1.BlockMetadataTransaction} returns this
 */
proto.aptos.transaction.testing1.v1.BlockMetadataTransaction.prototype.clearFailedProposerIndicesList = function() {
  return this.setFailedProposerIndicesList([]);
};



/**
 * List of repeated fields within this message type.
 * @private {!Array<number>}
 * @const
 */
proto.aptos.transaction.testing1.v1.GenesisTransaction.repeatedFields_ = [2];



if (jspb.Message.GENERATE_TO_OBJECT) {
/**
 * Creates an object representation of this proto.
 * Field names that are reserved in JavaScript and will be renamed to pb_name.
 * Optional fields that are not set will be set to undefined.
 * To access a reserved field use, foo.pb_<name>, eg, foo.pb_default.
 * For the list of reserved names please see:
 *     net/proto2/compiler/js/internal/generator.cc#kKeyword.
 * @param {boolean=} opt_includeInstance Deprecated. whether to include the
 *     JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @return {!Object}
 */
proto.aptos.transaction.testing1.v1.GenesisTransaction.prototype.toObject = function(opt_includeInstance) {
  return proto.aptos.transaction.testing1.v1.GenesisTransaction.toObject(opt_includeInstance, this);
};


/**
 * Static version of the {@see toObject} method.
 * @param {boolean|undefined} includeInstance Deprecated. Whether to include
 *     the JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @param {!proto.aptos.transaction.testing1.v1.GenesisTransaction} msg The msg instance to transform.
 * @return {!Object}
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.aptos.transaction.testing1.v1.GenesisTransaction.toObject = function(includeInstance, msg) {
  var f, obj = {
    payload: (f = msg.getPayload()) && proto.aptos.transaction.testing1.v1.WriteSet.toObject(includeInstance, f),
    eventsList: jspb.Message.toObjectList(msg.getEventsList(),
    proto.aptos.transaction.testing1.v1.Event.toObject, includeInstance)
  };

  if (includeInstance) {
    obj.$jspbMessageInstance = msg;
  }
  return obj;
};
}


/**
 * Deserializes binary data (in protobuf wire format).
 * @param {jspb.ByteSource} bytes The bytes to deserialize.
 * @return {!proto.aptos.transaction.testing1.v1.GenesisTransaction}
 */
proto.aptos.transaction.testing1.v1.GenesisTransaction.deserializeBinary = function(bytes) {
  var reader = new jspb.BinaryReader(bytes);
  var msg = new proto.aptos.transaction.testing1.v1.GenesisTransaction;
  return proto.aptos.transaction.testing1.v1.GenesisTransaction.deserializeBinaryFromReader(msg, reader);
};


/**
 * Deserializes binary data (in protobuf wire format) from the
 * given reader into the given message object.
 * @param {!proto.aptos.transaction.testing1.v1.GenesisTransaction} msg The message object to deserialize into.
 * @param {!jspb.BinaryReader} reader The BinaryReader to use.
 * @return {!proto.aptos.transaction.testing1.v1.GenesisTransaction}
 */
proto.aptos.transaction.testing1.v1.GenesisTransaction.deserializeBinaryFromReader = function(msg, reader) {
  while (reader.nextField()) {
    if (reader.isEndGroup()) {
      break;
    }
    var field = reader.getFieldNumber();
    switch (field) {
    case 1:
      var value = new proto.aptos.transaction.testing1.v1.WriteSet;
      reader.readMessage(value,proto.aptos.transaction.testing1.v1.WriteSet.deserializeBinaryFromReader);
      msg.setPayload(value);
      break;
    case 2:
      var value = new proto.aptos.transaction.testing1.v1.Event;
      reader.readMessage(value,proto.aptos.transaction.testing1.v1.Event.deserializeBinaryFromReader);
      msg.addEvents(value);
      break;
    default:
      reader.skipField();
      break;
    }
  }
  return msg;
};


/**
 * Serializes the message to binary data (in protobuf wire format).
 * @return {!Uint8Array}
 */
proto.aptos.transaction.testing1.v1.GenesisTransaction.prototype.serializeBinary = function() {
  var writer = new jspb.BinaryWriter();
  proto.aptos.transaction.testing1.v1.GenesisTransaction.serializeBinaryToWriter(this, writer);
  return writer.getResultBuffer();
};


/**
 * Serializes the given message to binary data (in protobuf wire
 * format), writing to the given BinaryWriter.
 * @param {!proto.aptos.transaction.testing1.v1.GenesisTransaction} message
 * @param {!jspb.BinaryWriter} writer
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.aptos.transaction.testing1.v1.GenesisTransaction.serializeBinaryToWriter = function(message, writer) {
  var f = undefined;
  f = message.getPayload();
  if (f != null) {
    writer.writeMessage(
      1,
      f,
      proto.aptos.transaction.testing1.v1.WriteSet.serializeBinaryToWriter
    );
  }
  f = message.getEventsList();
  if (f.length > 0) {
    writer.writeRepeatedMessage(
      2,
      f,
      proto.aptos.transaction.testing1.v1.Event.serializeBinaryToWriter
    );
  }
};


/**
 * optional WriteSet payload = 1;
 * @return {?proto.aptos.transaction.testing1.v1.WriteSet}
 */
proto.aptos.transaction.testing1.v1.GenesisTransaction.prototype.getPayload = function() {
  return /** @type{?proto.aptos.transaction.testing1.v1.WriteSet} */ (
    jspb.Message.getWrapperField(this, proto.aptos.transaction.testing1.v1.WriteSet, 1));
};


/**
 * @param {?proto.aptos.transaction.testing1.v1.WriteSet|undefined} value
 * @return {!proto.aptos.transaction.testing1.v1.GenesisTransaction} returns this
*/
proto.aptos.transaction.testing1.v1.GenesisTransaction.prototype.setPayload = function(value) {
  return jspb.Message.setWrapperField(this, 1, value);
};


/**
 * Clears the message field making it undefined.
 * @return {!proto.aptos.transaction.testing1.v1.GenesisTransaction} returns this
 */
proto.aptos.transaction.testing1.v1.GenesisTransaction.prototype.clearPayload = function() {
  return this.setPayload(undefined);
};


/**
 * Returns whether this field is set.
 * @return {boolean}
 */
proto.aptos.transaction.testing1.v1.GenesisTransaction.prototype.hasPayload = function() {
  return jspb.Message.getField(this, 1) != null;
};


/**
 * repeated Event events = 2;
 * @return {!Array<!proto.aptos.transaction.testing1.v1.Event>}
 */
proto.aptos.transaction.testing1.v1.GenesisTransaction.prototype.getEventsList = function() {
  return /** @type{!Array<!proto.aptos.transaction.testing1.v1.Event>} */ (
    jspb.Message.getRepeatedWrapperField(this, proto.aptos.transaction.testing1.v1.Event, 2));
};


/**
 * @param {!Array<!proto.aptos.transaction.testing1.v1.Event>} value
 * @return {!proto.aptos.transaction.testing1.v1.GenesisTransaction} returns this
*/
proto.aptos.transaction.testing1.v1.GenesisTransaction.prototype.setEventsList = function(value) {
  return jspb.Message.setRepeatedWrapperField(this, 2, value);
};


/**
 * @param {!proto.aptos.transaction.testing1.v1.Event=} opt_value
 * @param {number=} opt_index
 * @return {!proto.aptos.transaction.testing1.v1.Event}
 */
proto.aptos.transaction.testing1.v1.GenesisTransaction.prototype.addEvents = function(opt_value, opt_index) {
  return jspb.Message.addToRepeatedWrapperField(this, 2, opt_value, proto.aptos.transaction.testing1.v1.Event, opt_index);
};


/**
 * Clears the list making it empty but non-null.
 * @return {!proto.aptos.transaction.testing1.v1.GenesisTransaction} returns this
 */
proto.aptos.transaction.testing1.v1.GenesisTransaction.prototype.clearEventsList = function() {
  return this.setEventsList([]);
};





if (jspb.Message.GENERATE_TO_OBJECT) {
/**
 * Creates an object representation of this proto.
 * Field names that are reserved in JavaScript and will be renamed to pb_name.
 * Optional fields that are not set will be set to undefined.
 * To access a reserved field use, foo.pb_<name>, eg, foo.pb_default.
 * For the list of reserved names please see:
 *     net/proto2/compiler/js/internal/generator.cc#kKeyword.
 * @param {boolean=} opt_includeInstance Deprecated. whether to include the
 *     JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @return {!Object}
 */
proto.aptos.transaction.testing1.v1.StateCheckpointTransaction.prototype.toObject = function(opt_includeInstance) {
  return proto.aptos.transaction.testing1.v1.StateCheckpointTransaction.toObject(opt_includeInstance, this);
};


/**
 * Static version of the {@see toObject} method.
 * @param {boolean|undefined} includeInstance Deprecated. Whether to include
 *     the JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @param {!proto.aptos.transaction.testing1.v1.StateCheckpointTransaction} msg The msg instance to transform.
 * @return {!Object}
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.aptos.transaction.testing1.v1.StateCheckpointTransaction.toObject = function(includeInstance, msg) {
  var f, obj = {

  };

  if (includeInstance) {
    obj.$jspbMessageInstance = msg;
  }
  return obj;
};
}


/**
 * Deserializes binary data (in protobuf wire format).
 * @param {jspb.ByteSource} bytes The bytes to deserialize.
 * @return {!proto.aptos.transaction.testing1.v1.StateCheckpointTransaction}
 */
proto.aptos.transaction.testing1.v1.StateCheckpointTransaction.deserializeBinary = function(bytes) {
  var reader = new jspb.BinaryReader(bytes);
  var msg = new proto.aptos.transaction.testing1.v1.StateCheckpointTransaction;
  return proto.aptos.transaction.testing1.v1.StateCheckpointTransaction.deserializeBinaryFromReader(msg, reader);
};


/**
 * Deserializes binary data (in protobuf wire format) from the
 * given reader into the given message object.
 * @param {!proto.aptos.transaction.testing1.v1.StateCheckpointTransaction} msg The message object to deserialize into.
 * @param {!jspb.BinaryReader} reader The BinaryReader to use.
 * @return {!proto.aptos.transaction.testing1.v1.StateCheckpointTransaction}
 */
proto.aptos.transaction.testing1.v1.StateCheckpointTransaction.deserializeBinaryFromReader = function(msg, reader) {
  while (reader.nextField()) {
    if (reader.isEndGroup()) {
      break;
    }
    var field = reader.getFieldNumber();
    switch (field) {
    default:
      reader.skipField();
      break;
    }
  }
  return msg;
};


/**
 * Serializes the message to binary data (in protobuf wire format).
 * @return {!Uint8Array}
 */
proto.aptos.transaction.testing1.v1.StateCheckpointTransaction.prototype.serializeBinary = function() {
  var writer = new jspb.BinaryWriter();
  proto.aptos.transaction.testing1.v1.StateCheckpointTransaction.serializeBinaryToWriter(this, writer);
  return writer.getResultBuffer();
};


/**
 * Serializes the given message to binary data (in protobuf wire
 * format), writing to the given BinaryWriter.
 * @param {!proto.aptos.transaction.testing1.v1.StateCheckpointTransaction} message
 * @param {!jspb.BinaryWriter} writer
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.aptos.transaction.testing1.v1.StateCheckpointTransaction.serializeBinaryToWriter = function(message, writer) {
  var f = undefined;
};



/**
 * List of repeated fields within this message type.
 * @private {!Array<number>}
 * @const
 */
proto.aptos.transaction.testing1.v1.UserTransaction.repeatedFields_ = [2];



if (jspb.Message.GENERATE_TO_OBJECT) {
/**
 * Creates an object representation of this proto.
 * Field names that are reserved in JavaScript and will be renamed to pb_name.
 * Optional fields that are not set will be set to undefined.
 * To access a reserved field use, foo.pb_<name>, eg, foo.pb_default.
 * For the list of reserved names please see:
 *     net/proto2/compiler/js/internal/generator.cc#kKeyword.
 * @param {boolean=} opt_includeInstance Deprecated. whether to include the
 *     JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @return {!Object}
 */
proto.aptos.transaction.testing1.v1.UserTransaction.prototype.toObject = function(opt_includeInstance) {
  return proto.aptos.transaction.testing1.v1.UserTransaction.toObject(opt_includeInstance, this);
};


/**
 * Static version of the {@see toObject} method.
 * @param {boolean|undefined} includeInstance Deprecated. Whether to include
 *     the JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @param {!proto.aptos.transaction.testing1.v1.UserTransaction} msg The msg instance to transform.
 * @return {!Object}
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.aptos.transaction.testing1.v1.UserTransaction.toObject = function(includeInstance, msg) {
  var f, obj = {
    request: (f = msg.getRequest()) && proto.aptos.transaction.testing1.v1.UserTransactionRequest.toObject(includeInstance, f),
    eventsList: jspb.Message.toObjectList(msg.getEventsList(),
    proto.aptos.transaction.testing1.v1.Event.toObject, includeInstance)
  };

  if (includeInstance) {
    obj.$jspbMessageInstance = msg;
  }
  return obj;
};
}


/**
 * Deserializes binary data (in protobuf wire format).
 * @param {jspb.ByteSource} bytes The bytes to deserialize.
 * @return {!proto.aptos.transaction.testing1.v1.UserTransaction}
 */
proto.aptos.transaction.testing1.v1.UserTransaction.deserializeBinary = function(bytes) {
  var reader = new jspb.BinaryReader(bytes);
  var msg = new proto.aptos.transaction.testing1.v1.UserTransaction;
  return proto.aptos.transaction.testing1.v1.UserTransaction.deserializeBinaryFromReader(msg, reader);
};


/**
 * Deserializes binary data (in protobuf wire format) from the
 * given reader into the given message object.
 * @param {!proto.aptos.transaction.testing1.v1.UserTransaction} msg The message object to deserialize into.
 * @param {!jspb.BinaryReader} reader The BinaryReader to use.
 * @return {!proto.aptos.transaction.testing1.v1.UserTransaction}
 */
proto.aptos.transaction.testing1.v1.UserTransaction.deserializeBinaryFromReader = function(msg, reader) {
  while (reader.nextField()) {
    if (reader.isEndGroup()) {
      break;
    }
    var field = reader.getFieldNumber();
    switch (field) {
    case 1:
      var value = new proto.aptos.transaction.testing1.v1.UserTransactionRequest;
      reader.readMessage(value,proto.aptos.transaction.testing1.v1.UserTransactionRequest.deserializeBinaryFromReader);
      msg.setRequest(value);
      break;
    case 2:
      var value = new proto.aptos.transaction.testing1.v1.Event;
      reader.readMessage(value,proto.aptos.transaction.testing1.v1.Event.deserializeBinaryFromReader);
      msg.addEvents(value);
      break;
    default:
      reader.skipField();
      break;
    }
  }
  return msg;
};


/**
 * Serializes the message to binary data (in protobuf wire format).
 * @return {!Uint8Array}
 */
proto.aptos.transaction.testing1.v1.UserTransaction.prototype.serializeBinary = function() {
  var writer = new jspb.BinaryWriter();
  proto.aptos.transaction.testing1.v1.UserTransaction.serializeBinaryToWriter(this, writer);
  return writer.getResultBuffer();
};


/**
 * Serializes the given message to binary data (in protobuf wire
 * format), writing to the given BinaryWriter.
 * @param {!proto.aptos.transaction.testing1.v1.UserTransaction} message
 * @param {!jspb.BinaryWriter} writer
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.aptos.transaction.testing1.v1.UserTransaction.serializeBinaryToWriter = function(message, writer) {
  var f = undefined;
  f = message.getRequest();
  if (f != null) {
    writer.writeMessage(
      1,
      f,
      proto.aptos.transaction.testing1.v1.UserTransactionRequest.serializeBinaryToWriter
    );
  }
  f = message.getEventsList();
  if (f.length > 0) {
    writer.writeRepeatedMessage(
      2,
      f,
      proto.aptos.transaction.testing1.v1.Event.serializeBinaryToWriter
    );
  }
};


/**
 * optional UserTransactionRequest request = 1;
 * @return {?proto.aptos.transaction.testing1.v1.UserTransactionRequest}
 */
proto.aptos.transaction.testing1.v1.UserTransaction.prototype.getRequest = function() {
  return /** @type{?proto.aptos.transaction.testing1.v1.UserTransactionRequest} */ (
    jspb.Message.getWrapperField(this, proto.aptos.transaction.testing1.v1.UserTransactionRequest, 1));
};


/**
 * @param {?proto.aptos.transaction.testing1.v1.UserTransactionRequest|undefined} value
 * @return {!proto.aptos.transaction.testing1.v1.UserTransaction} returns this
*/
proto.aptos.transaction.testing1.v1.UserTransaction.prototype.setRequest = function(value) {
  return jspb.Message.setWrapperField(this, 1, value);
};


/**
 * Clears the message field making it undefined.
 * @return {!proto.aptos.transaction.testing1.v1.UserTransaction} returns this
 */
proto.aptos.transaction.testing1.v1.UserTransaction.prototype.clearRequest = function() {
  return this.setRequest(undefined);
};


/**
 * Returns whether this field is set.
 * @return {boolean}
 */
proto.aptos.transaction.testing1.v1.UserTransaction.prototype.hasRequest = function() {
  return jspb.Message.getField(this, 1) != null;
};


/**
 * repeated Event events = 2;
 * @return {!Array<!proto.aptos.transaction.testing1.v1.Event>}
 */
proto.aptos.transaction.testing1.v1.UserTransaction.prototype.getEventsList = function() {
  return /** @type{!Array<!proto.aptos.transaction.testing1.v1.Event>} */ (
    jspb.Message.getRepeatedWrapperField(this, proto.aptos.transaction.testing1.v1.Event, 2));
};


/**
 * @param {!Array<!proto.aptos.transaction.testing1.v1.Event>} value
 * @return {!proto.aptos.transaction.testing1.v1.UserTransaction} returns this
*/
proto.aptos.transaction.testing1.v1.UserTransaction.prototype.setEventsList = function(value) {
  return jspb.Message.setRepeatedWrapperField(this, 2, value);
};


/**
 * @param {!proto.aptos.transaction.testing1.v1.Event=} opt_value
 * @param {number=} opt_index
 * @return {!proto.aptos.transaction.testing1.v1.Event}
 */
proto.aptos.transaction.testing1.v1.UserTransaction.prototype.addEvents = function(opt_value, opt_index) {
  return jspb.Message.addToRepeatedWrapperField(this, 2, opt_value, proto.aptos.transaction.testing1.v1.Event, opt_index);
};


/**
 * Clears the list making it empty but non-null.
 * @return {!proto.aptos.transaction.testing1.v1.UserTransaction} returns this
 */
proto.aptos.transaction.testing1.v1.UserTransaction.prototype.clearEventsList = function() {
  return this.setEventsList([]);
};





if (jspb.Message.GENERATE_TO_OBJECT) {
/**
 * Creates an object representation of this proto.
 * Field names that are reserved in JavaScript and will be renamed to pb_name.
 * Optional fields that are not set will be set to undefined.
 * To access a reserved field use, foo.pb_<name>, eg, foo.pb_default.
 * For the list of reserved names please see:
 *     net/proto2/compiler/js/internal/generator.cc#kKeyword.
 * @param {boolean=} opt_includeInstance Deprecated. whether to include the
 *     JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @return {!Object}
 */
proto.aptos.transaction.testing1.v1.Event.prototype.toObject = function(opt_includeInstance) {
  return proto.aptos.transaction.testing1.v1.Event.toObject(opt_includeInstance, this);
};


/**
 * Static version of the {@see toObject} method.
 * @param {boolean|undefined} includeInstance Deprecated. Whether to include
 *     the JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @param {!proto.aptos.transaction.testing1.v1.Event} msg The msg instance to transform.
 * @return {!Object}
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.aptos.transaction.testing1.v1.Event.toObject = function(includeInstance, msg) {
  var f, obj = {
    key: (f = msg.getKey()) && proto.aptos.transaction.testing1.v1.EventKey.toObject(includeInstance, f),
    sequenceNumber: jspb.Message.getFieldWithDefault(msg, 2, 0),
    type: (f = msg.getType()) && proto.aptos.transaction.testing1.v1.MoveType.toObject(includeInstance, f),
    typeStr: jspb.Message.getFieldWithDefault(msg, 5, ""),
    data: jspb.Message.getFieldWithDefault(msg, 4, "")
  };

  if (includeInstance) {
    obj.$jspbMessageInstance = msg;
  }
  return obj;
};
}


/**
 * Deserializes binary data (in protobuf wire format).
 * @param {jspb.ByteSource} bytes The bytes to deserialize.
 * @return {!proto.aptos.transaction.testing1.v1.Event}
 */
proto.aptos.transaction.testing1.v1.Event.deserializeBinary = function(bytes) {
  var reader = new jspb.BinaryReader(bytes);
  var msg = new proto.aptos.transaction.testing1.v1.Event;
  return proto.aptos.transaction.testing1.v1.Event.deserializeBinaryFromReader(msg, reader);
};


/**
 * Deserializes binary data (in protobuf wire format) from the
 * given reader into the given message object.
 * @param {!proto.aptos.transaction.testing1.v1.Event} msg The message object to deserialize into.
 * @param {!jspb.BinaryReader} reader The BinaryReader to use.
 * @return {!proto.aptos.transaction.testing1.v1.Event}
 */
proto.aptos.transaction.testing1.v1.Event.deserializeBinaryFromReader = function(msg, reader) {
  while (reader.nextField()) {
    if (reader.isEndGroup()) {
      break;
    }
    var field = reader.getFieldNumber();
    switch (field) {
    case 1:
      var value = new proto.aptos.transaction.testing1.v1.EventKey;
      reader.readMessage(value,proto.aptos.transaction.testing1.v1.EventKey.deserializeBinaryFromReader);
      msg.setKey(value);
      break;
    case 2:
      var value = /** @type {number} */ (reader.readUint64());
      msg.setSequenceNumber(value);
      break;
    case 3:
      var value = new proto.aptos.transaction.testing1.v1.MoveType;
      reader.readMessage(value,proto.aptos.transaction.testing1.v1.MoveType.deserializeBinaryFromReader);
      msg.setType(value);
      break;
    case 5:
      var value = /** @type {string} */ (reader.readString());
      msg.setTypeStr(value);
      break;
    case 4:
      var value = /** @type {string} */ (reader.readString());
      msg.setData(value);
      break;
    default:
      reader.skipField();
      break;
    }
  }
  return msg;
};


/**
 * Serializes the message to binary data (in protobuf wire format).
 * @return {!Uint8Array}
 */
proto.aptos.transaction.testing1.v1.Event.prototype.serializeBinary = function() {
  var writer = new jspb.BinaryWriter();
  proto.aptos.transaction.testing1.v1.Event.serializeBinaryToWriter(this, writer);
  return writer.getResultBuffer();
};


/**
 * Serializes the given message to binary data (in protobuf wire
 * format), writing to the given BinaryWriter.
 * @param {!proto.aptos.transaction.testing1.v1.Event} message
 * @param {!jspb.BinaryWriter} writer
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.aptos.transaction.testing1.v1.Event.serializeBinaryToWriter = function(message, writer) {
  var f = undefined;
  f = message.getKey();
  if (f != null) {
    writer.writeMessage(
      1,
      f,
      proto.aptos.transaction.testing1.v1.EventKey.serializeBinaryToWriter
    );
  }
  f = message.getSequenceNumber();
  if (f !== 0) {
    writer.writeUint64(
      2,
      f
    );
  }
  f = message.getType();
  if (f != null) {
    writer.writeMessage(
      3,
      f,
      proto.aptos.transaction.testing1.v1.MoveType.serializeBinaryToWriter
    );
  }
  f = message.getTypeStr();
  if (f.length > 0) {
    writer.writeString(
      5,
      f
    );
  }
  f = message.getData();
  if (f.length > 0) {
    writer.writeString(
      4,
      f
    );
  }
};


/**
 * optional EventKey key = 1;
 * @return {?proto.aptos.transaction.testing1.v1.EventKey}
 */
proto.aptos.transaction.testing1.v1.Event.prototype.getKey = function() {
  return /** @type{?proto.aptos.transaction.testing1.v1.EventKey} */ (
    jspb.Message.getWrapperField(this, proto.aptos.transaction.testing1.v1.EventKey, 1));
};


/**
 * @param {?proto.aptos.transaction.testing1.v1.EventKey|undefined} value
 * @return {!proto.aptos.transaction.testing1.v1.Event} returns this
*/
proto.aptos.transaction.testing1.v1.Event.prototype.setKey = function(value) {
  return jspb.Message.setWrapperField(this, 1, value);
};


/**
 * Clears the message field making it undefined.
 * @return {!proto.aptos.transaction.testing1.v1.Event} returns this
 */
proto.aptos.transaction.testing1.v1.Event.prototype.clearKey = function() {
  return this.setKey(undefined);
};


/**
 * Returns whether this field is set.
 * @return {boolean}
 */
proto.aptos.transaction.testing1.v1.Event.prototype.hasKey = function() {
  return jspb.Message.getField(this, 1) != null;
};


/**
 * optional uint64 sequence_number = 2;
 * @return {number}
 */
proto.aptos.transaction.testing1.v1.Event.prototype.getSequenceNumber = function() {
  return /** @type {number} */ (jspb.Message.getFieldWithDefault(this, 2, 0));
};


/**
 * @param {number} value
 * @return {!proto.aptos.transaction.testing1.v1.Event} returns this
 */
proto.aptos.transaction.testing1.v1.Event.prototype.setSequenceNumber = function(value) {
  return jspb.Message.setProto3IntField(this, 2, value);
};


/**
 * optional MoveType type = 3;
 * @return {?proto.aptos.transaction.testing1.v1.MoveType}
 */
proto.aptos.transaction.testing1.v1.Event.prototype.getType = function() {
  return /** @type{?proto.aptos.transaction.testing1.v1.MoveType} */ (
    jspb.Message.getWrapperField(this, proto.aptos.transaction.testing1.v1.MoveType, 3));
};


/**
 * @param {?proto.aptos.transaction.testing1.v1.MoveType|undefined} value
 * @return {!proto.aptos.transaction.testing1.v1.Event} returns this
*/
proto.aptos.transaction.testing1.v1.Event.prototype.setType = function(value) {
  return jspb.Message.setWrapperField(this, 3, value);
};


/**
 * Clears the message field making it undefined.
 * @return {!proto.aptos.transaction.testing1.v1.Event} returns this
 */
proto.aptos.transaction.testing1.v1.Event.prototype.clearType = function() {
  return this.setType(undefined);
};


/**
 * Returns whether this field is set.
 * @return {boolean}
 */
proto.aptos.transaction.testing1.v1.Event.prototype.hasType = function() {
  return jspb.Message.getField(this, 3) != null;
};


/**
 * optional string type_str = 5;
 * @return {string}
 */
proto.aptos.transaction.testing1.v1.Event.prototype.getTypeStr = function() {
  return /** @type {string} */ (jspb.Message.getFieldWithDefault(this, 5, ""));
};


/**
 * @param {string} value
 * @return {!proto.aptos.transaction.testing1.v1.Event} returns this
 */
proto.aptos.transaction.testing1.v1.Event.prototype.setTypeStr = function(value) {
  return jspb.Message.setProto3StringField(this, 5, value);
};


/**
 * optional string data = 4;
 * @return {string}
 */
proto.aptos.transaction.testing1.v1.Event.prototype.getData = function() {
  return /** @type {string} */ (jspb.Message.getFieldWithDefault(this, 4, ""));
};


/**
 * @param {string} value
 * @return {!proto.aptos.transaction.testing1.v1.Event} returns this
 */
proto.aptos.transaction.testing1.v1.Event.prototype.setData = function(value) {
  return jspb.Message.setProto3StringField(this, 4, value);
};



/**
 * List of repeated fields within this message type.
 * @private {!Array<number>}
 * @const
 */
proto.aptos.transaction.testing1.v1.TransactionInfo.repeatedFields_ = [9];



if (jspb.Message.GENERATE_TO_OBJECT) {
/**
 * Creates an object representation of this proto.
 * Field names that are reserved in JavaScript and will be renamed to pb_name.
 * Optional fields that are not set will be set to undefined.
 * To access a reserved field use, foo.pb_<name>, eg, foo.pb_default.
 * For the list of reserved names please see:
 *     net/proto2/compiler/js/internal/generator.cc#kKeyword.
 * @param {boolean=} opt_includeInstance Deprecated. whether to include the
 *     JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @return {!Object}
 */
proto.aptos.transaction.testing1.v1.TransactionInfo.prototype.toObject = function(opt_includeInstance) {
  return proto.aptos.transaction.testing1.v1.TransactionInfo.toObject(opt_includeInstance, this);
};


/**
 * Static version of the {@see toObject} method.
 * @param {boolean|undefined} includeInstance Deprecated. Whether to include
 *     the JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @param {!proto.aptos.transaction.testing1.v1.TransactionInfo} msg The msg instance to transform.
 * @return {!Object}
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.aptos.transaction.testing1.v1.TransactionInfo.toObject = function(includeInstance, msg) {
  var f, obj = {
    hash: msg.getHash_asB64(),
    stateChangeHash: msg.getStateChangeHash_asB64(),
    eventRootHash: msg.getEventRootHash_asB64(),
    stateCheckpointHash: msg.getStateCheckpointHash_asB64(),
    gasUsed: jspb.Message.getFieldWithDefault(msg, 5, 0),
    success: jspb.Message.getBooleanFieldWithDefault(msg, 6, false),
    vmStatus: jspb.Message.getFieldWithDefault(msg, 7, ""),
    accumulatorRootHash: msg.getAccumulatorRootHash_asB64(),
    changesList: jspb.Message.toObjectList(msg.getChangesList(),
    proto.aptos.transaction.testing1.v1.WriteSetChange.toObject, includeInstance)
  };

  if (includeInstance) {
    obj.$jspbMessageInstance = msg;
  }
  return obj;
};
}


/**
 * Deserializes binary data (in protobuf wire format).
 * @param {jspb.ByteSource} bytes The bytes to deserialize.
 * @return {!proto.aptos.transaction.testing1.v1.TransactionInfo}
 */
proto.aptos.transaction.testing1.v1.TransactionInfo.deserializeBinary = function(bytes) {
  var reader = new jspb.BinaryReader(bytes);
  var msg = new proto.aptos.transaction.testing1.v1.TransactionInfo;
  return proto.aptos.transaction.testing1.v1.TransactionInfo.deserializeBinaryFromReader(msg, reader);
};


/**
 * Deserializes binary data (in protobuf wire format) from the
 * given reader into the given message object.
 * @param {!proto.aptos.transaction.testing1.v1.TransactionInfo} msg The message object to deserialize into.
 * @param {!jspb.BinaryReader} reader The BinaryReader to use.
 * @return {!proto.aptos.transaction.testing1.v1.TransactionInfo}
 */
proto.aptos.transaction.testing1.v1.TransactionInfo.deserializeBinaryFromReader = function(msg, reader) {
  while (reader.nextField()) {
    if (reader.isEndGroup()) {
      break;
    }
    var field = reader.getFieldNumber();
    switch (field) {
    case 1:
      var value = /** @type {!Uint8Array} */ (reader.readBytes());
      msg.setHash(value);
      break;
    case 2:
      var value = /** @type {!Uint8Array} */ (reader.readBytes());
      msg.setStateChangeHash(value);
      break;
    case 3:
      var value = /** @type {!Uint8Array} */ (reader.readBytes());
      msg.setEventRootHash(value);
      break;
    case 4:
      var value = /** @type {!Uint8Array} */ (reader.readBytes());
      msg.setStateCheckpointHash(value);
      break;
    case 5:
      var value = /** @type {number} */ (reader.readUint64());
      msg.setGasUsed(value);
      break;
    case 6:
      var value = /** @type {boolean} */ (reader.readBool());
      msg.setSuccess(value);
      break;
    case 7:
      var value = /** @type {string} */ (reader.readString());
      msg.setVmStatus(value);
      break;
    case 8:
      var value = /** @type {!Uint8Array} */ (reader.readBytes());
      msg.setAccumulatorRootHash(value);
      break;
    case 9:
      var value = new proto.aptos.transaction.testing1.v1.WriteSetChange;
      reader.readMessage(value,proto.aptos.transaction.testing1.v1.WriteSetChange.deserializeBinaryFromReader);
      msg.addChanges(value);
      break;
    default:
      reader.skipField();
      break;
    }
  }
  return msg;
};


/**
 * Serializes the message to binary data (in protobuf wire format).
 * @return {!Uint8Array}
 */
proto.aptos.transaction.testing1.v1.TransactionInfo.prototype.serializeBinary = function() {
  var writer = new jspb.BinaryWriter();
  proto.aptos.transaction.testing1.v1.TransactionInfo.serializeBinaryToWriter(this, writer);
  return writer.getResultBuffer();
};


/**
 * Serializes the given message to binary data (in protobuf wire
 * format), writing to the given BinaryWriter.
 * @param {!proto.aptos.transaction.testing1.v1.TransactionInfo} message
 * @param {!jspb.BinaryWriter} writer
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.aptos.transaction.testing1.v1.TransactionInfo.serializeBinaryToWriter = function(message, writer) {
  var f = undefined;
  f = message.getHash_asU8();
  if (f.length > 0) {
    writer.writeBytes(
      1,
      f
    );
  }
  f = message.getStateChangeHash_asU8();
  if (f.length > 0) {
    writer.writeBytes(
      2,
      f
    );
  }
  f = message.getEventRootHash_asU8();
  if (f.length > 0) {
    writer.writeBytes(
      3,
      f
    );
  }
  f = /** @type {!(string|Uint8Array)} */ (jspb.Message.getField(message, 4));
  if (f != null) {
    writer.writeBytes(
      4,
      f
    );
  }
  f = message.getGasUsed();
  if (f !== 0) {
    writer.writeUint64(
      5,
      f
    );
  }
  f = message.getSuccess();
  if (f) {
    writer.writeBool(
      6,
      f
    );
  }
  f = message.getVmStatus();
  if (f.length > 0) {
    writer.writeString(
      7,
      f
    );
  }
  f = message.getAccumulatorRootHash_asU8();
  if (f.length > 0) {
    writer.writeBytes(
      8,
      f
    );
  }
  f = message.getChangesList();
  if (f.length > 0) {
    writer.writeRepeatedMessage(
      9,
      f,
      proto.aptos.transaction.testing1.v1.WriteSetChange.serializeBinaryToWriter
    );
  }
};


/**
 * optional bytes hash = 1;
 * @return {!(string|Uint8Array)}
 */
proto.aptos.transaction.testing1.v1.TransactionInfo.prototype.getHash = function() {
  return /** @type {!(string|Uint8Array)} */ (jspb.Message.getFieldWithDefault(this, 1, ""));
};


/**
 * optional bytes hash = 1;
 * This is a type-conversion wrapper around `getHash()`
 * @return {string}
 */
proto.aptos.transaction.testing1.v1.TransactionInfo.prototype.getHash_asB64 = function() {
  return /** @type {string} */ (jspb.Message.bytesAsB64(
      this.getHash()));
};


/**
 * optional bytes hash = 1;
 * Note that Uint8Array is not supported on all browsers.
 * @see http://caniuse.com/Uint8Array
 * This is a type-conversion wrapper around `getHash()`
 * @return {!Uint8Array}
 */
proto.aptos.transaction.testing1.v1.TransactionInfo.prototype.getHash_asU8 = function() {
  return /** @type {!Uint8Array} */ (jspb.Message.bytesAsU8(
      this.getHash()));
};


/**
 * @param {!(string|Uint8Array)} value
 * @return {!proto.aptos.transaction.testing1.v1.TransactionInfo} returns this
 */
proto.aptos.transaction.testing1.v1.TransactionInfo.prototype.setHash = function(value) {
  return jspb.Message.setProto3BytesField(this, 1, value);
};


/**
 * optional bytes state_change_hash = 2;
 * @return {!(string|Uint8Array)}
 */
proto.aptos.transaction.testing1.v1.TransactionInfo.prototype.getStateChangeHash = function() {
  return /** @type {!(string|Uint8Array)} */ (jspb.Message.getFieldWithDefault(this, 2, ""));
};


/**
 * optional bytes state_change_hash = 2;
 * This is a type-conversion wrapper around `getStateChangeHash()`
 * @return {string}
 */
proto.aptos.transaction.testing1.v1.TransactionInfo.prototype.getStateChangeHash_asB64 = function() {
  return /** @type {string} */ (jspb.Message.bytesAsB64(
      this.getStateChangeHash()));
};


/**
 * optional bytes state_change_hash = 2;
 * Note that Uint8Array is not supported on all browsers.
 * @see http://caniuse.com/Uint8Array
 * This is a type-conversion wrapper around `getStateChangeHash()`
 * @return {!Uint8Array}
 */
proto.aptos.transaction.testing1.v1.TransactionInfo.prototype.getStateChangeHash_asU8 = function() {
  return /** @type {!Uint8Array} */ (jspb.Message.bytesAsU8(
      this.getStateChangeHash()));
};


/**
 * @param {!(string|Uint8Array)} value
 * @return {!proto.aptos.transaction.testing1.v1.TransactionInfo} returns this
 */
proto.aptos.transaction.testing1.v1.TransactionInfo.prototype.setStateChangeHash = function(value) {
  return jspb.Message.setProto3BytesField(this, 2, value);
};


/**
 * optional bytes event_root_hash = 3;
 * @return {!(string|Uint8Array)}
 */
proto.aptos.transaction.testing1.v1.TransactionInfo.prototype.getEventRootHash = function() {
  return /** @type {!(string|Uint8Array)} */ (jspb.Message.getFieldWithDefault(this, 3, ""));
};


/**
 * optional bytes event_root_hash = 3;
 * This is a type-conversion wrapper around `getEventRootHash()`
 * @return {string}
 */
proto.aptos.transaction.testing1.v1.TransactionInfo.prototype.getEventRootHash_asB64 = function() {
  return /** @type {string} */ (jspb.Message.bytesAsB64(
      this.getEventRootHash()));
};


/**
 * optional bytes event_root_hash = 3;
 * Note that Uint8Array is not supported on all browsers.
 * @see http://caniuse.com/Uint8Array
 * This is a type-conversion wrapper around `getEventRootHash()`
 * @return {!Uint8Array}
 */
proto.aptos.transaction.testing1.v1.TransactionInfo.prototype.getEventRootHash_asU8 = function() {
  return /** @type {!Uint8Array} */ (jspb.Message.bytesAsU8(
      this.getEventRootHash()));
};


/**
 * @param {!(string|Uint8Array)} value
 * @return {!proto.aptos.transaction.testing1.v1.TransactionInfo} returns this
 */
proto.aptos.transaction.testing1.v1.TransactionInfo.prototype.setEventRootHash = function(value) {
  return jspb.Message.setProto3BytesField(this, 3, value);
};


/**
 * optional bytes state_checkpoint_hash = 4;
 * @return {!(string|Uint8Array)}
 */
proto.aptos.transaction.testing1.v1.TransactionInfo.prototype.getStateCheckpointHash = function() {
  return /** @type {!(string|Uint8Array)} */ (jspb.Message.getFieldWithDefault(this, 4, ""));
};


/**
 * optional bytes state_checkpoint_hash = 4;
 * This is a type-conversion wrapper around `getStateCheckpointHash()`
 * @return {string}
 */
proto.aptos.transaction.testing1.v1.TransactionInfo.prototype.getStateCheckpointHash_asB64 = function() {
  return /** @type {string} */ (jspb.Message.bytesAsB64(
      this.getStateCheckpointHash()));
};


/**
 * optional bytes state_checkpoint_hash = 4;
 * Note that Uint8Array is not supported on all browsers.
 * @see http://caniuse.com/Uint8Array
 * This is a type-conversion wrapper around `getStateCheckpointHash()`
 * @return {!Uint8Array}
 */
proto.aptos.transaction.testing1.v1.TransactionInfo.prototype.getStateCheckpointHash_asU8 = function() {
  return /** @type {!Uint8Array} */ (jspb.Message.bytesAsU8(
      this.getStateCheckpointHash()));
};


/**
 * @param {!(string|Uint8Array)} value
 * @return {!proto.aptos.transaction.testing1.v1.TransactionInfo} returns this
 */
proto.aptos.transaction.testing1.v1.TransactionInfo.prototype.setStateCheckpointHash = function(value) {
  return jspb.Message.setField(this, 4, value);
};


/**
 * Clears the field making it undefined.
 * @return {!proto.aptos.transaction.testing1.v1.TransactionInfo} returns this
 */
proto.aptos.transaction.testing1.v1.TransactionInfo.prototype.clearStateCheckpointHash = function() {
  return jspb.Message.setField(this, 4, undefined);
};


/**
 * Returns whether this field is set.
 * @return {boolean}
 */
proto.aptos.transaction.testing1.v1.TransactionInfo.prototype.hasStateCheckpointHash = function() {
  return jspb.Message.getField(this, 4) != null;
};


/**
 * optional uint64 gas_used = 5;
 * @return {number}
 */
proto.aptos.transaction.testing1.v1.TransactionInfo.prototype.getGasUsed = function() {
  return /** @type {number} */ (jspb.Message.getFieldWithDefault(this, 5, 0));
};


/**
 * @param {number} value
 * @return {!proto.aptos.transaction.testing1.v1.TransactionInfo} returns this
 */
proto.aptos.transaction.testing1.v1.TransactionInfo.prototype.setGasUsed = function(value) {
  return jspb.Message.setProto3IntField(this, 5, value);
};


/**
 * optional bool success = 6;
 * @return {boolean}
 */
proto.aptos.transaction.testing1.v1.TransactionInfo.prototype.getSuccess = function() {
  return /** @type {boolean} */ (jspb.Message.getBooleanFieldWithDefault(this, 6, false));
};


/**
 * @param {boolean} value
 * @return {!proto.aptos.transaction.testing1.v1.TransactionInfo} returns this
 */
proto.aptos.transaction.testing1.v1.TransactionInfo.prototype.setSuccess = function(value) {
  return jspb.Message.setProto3BooleanField(this, 6, value);
};


/**
 * optional string vm_status = 7;
 * @return {string}
 */
proto.aptos.transaction.testing1.v1.TransactionInfo.prototype.getVmStatus = function() {
  return /** @type {string} */ (jspb.Message.getFieldWithDefault(this, 7, ""));
};


/**
 * @param {string} value
 * @return {!proto.aptos.transaction.testing1.v1.TransactionInfo} returns this
 */
proto.aptos.transaction.testing1.v1.TransactionInfo.prototype.setVmStatus = function(value) {
  return jspb.Message.setProto3StringField(this, 7, value);
};


/**
 * optional bytes accumulator_root_hash = 8;
 * @return {!(string|Uint8Array)}
 */
proto.aptos.transaction.testing1.v1.TransactionInfo.prototype.getAccumulatorRootHash = function() {
  return /** @type {!(string|Uint8Array)} */ (jspb.Message.getFieldWithDefault(this, 8, ""));
};


/**
 * optional bytes accumulator_root_hash = 8;
 * This is a type-conversion wrapper around `getAccumulatorRootHash()`
 * @return {string}
 */
proto.aptos.transaction.testing1.v1.TransactionInfo.prototype.getAccumulatorRootHash_asB64 = function() {
  return /** @type {string} */ (jspb.Message.bytesAsB64(
      this.getAccumulatorRootHash()));
};


/**
 * optional bytes accumulator_root_hash = 8;
 * Note that Uint8Array is not supported on all browsers.
 * @see http://caniuse.com/Uint8Array
 * This is a type-conversion wrapper around `getAccumulatorRootHash()`
 * @return {!Uint8Array}
 */
proto.aptos.transaction.testing1.v1.TransactionInfo.prototype.getAccumulatorRootHash_asU8 = function() {
  return /** @type {!Uint8Array} */ (jspb.Message.bytesAsU8(
      this.getAccumulatorRootHash()));
};


/**
 * @param {!(string|Uint8Array)} value
 * @return {!proto.aptos.transaction.testing1.v1.TransactionInfo} returns this
 */
proto.aptos.transaction.testing1.v1.TransactionInfo.prototype.setAccumulatorRootHash = function(value) {
  return jspb.Message.setProto3BytesField(this, 8, value);
};


/**
 * repeated WriteSetChange changes = 9;
 * @return {!Array<!proto.aptos.transaction.testing1.v1.WriteSetChange>}
 */
proto.aptos.transaction.testing1.v1.TransactionInfo.prototype.getChangesList = function() {
  return /** @type{!Array<!proto.aptos.transaction.testing1.v1.WriteSetChange>} */ (
    jspb.Message.getRepeatedWrapperField(this, proto.aptos.transaction.testing1.v1.WriteSetChange, 9));
};


/**
 * @param {!Array<!proto.aptos.transaction.testing1.v1.WriteSetChange>} value
 * @return {!proto.aptos.transaction.testing1.v1.TransactionInfo} returns this
*/
proto.aptos.transaction.testing1.v1.TransactionInfo.prototype.setChangesList = function(value) {
  return jspb.Message.setRepeatedWrapperField(this, 9, value);
};


/**
 * @param {!proto.aptos.transaction.testing1.v1.WriteSetChange=} opt_value
 * @param {number=} opt_index
 * @return {!proto.aptos.transaction.testing1.v1.WriteSetChange}
 */
proto.aptos.transaction.testing1.v1.TransactionInfo.prototype.addChanges = function(opt_value, opt_index) {
  return jspb.Message.addToRepeatedWrapperField(this, 9, opt_value, proto.aptos.transaction.testing1.v1.WriteSetChange, opt_index);
};


/**
 * Clears the list making it empty but non-null.
 * @return {!proto.aptos.transaction.testing1.v1.TransactionInfo} returns this
 */
proto.aptos.transaction.testing1.v1.TransactionInfo.prototype.clearChangesList = function() {
  return this.setChangesList([]);
};





if (jspb.Message.GENERATE_TO_OBJECT) {
/**
 * Creates an object representation of this proto.
 * Field names that are reserved in JavaScript and will be renamed to pb_name.
 * Optional fields that are not set will be set to undefined.
 * To access a reserved field use, foo.pb_<name>, eg, foo.pb_default.
 * For the list of reserved names please see:
 *     net/proto2/compiler/js/internal/generator.cc#kKeyword.
 * @param {boolean=} opt_includeInstance Deprecated. whether to include the
 *     JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @return {!Object}
 */
proto.aptos.transaction.testing1.v1.EventKey.prototype.toObject = function(opt_includeInstance) {
  return proto.aptos.transaction.testing1.v1.EventKey.toObject(opt_includeInstance, this);
};


/**
 * Static version of the {@see toObject} method.
 * @param {boolean|undefined} includeInstance Deprecated. Whether to include
 *     the JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @param {!proto.aptos.transaction.testing1.v1.EventKey} msg The msg instance to transform.
 * @return {!Object}
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.aptos.transaction.testing1.v1.EventKey.toObject = function(includeInstance, msg) {
  var f, obj = {
    creationNumber: jspb.Message.getFieldWithDefault(msg, 1, 0),
    accountAddress: jspb.Message.getFieldWithDefault(msg, 2, "")
  };

  if (includeInstance) {
    obj.$jspbMessageInstance = msg;
  }
  return obj;
};
}


/**
 * Deserializes binary data (in protobuf wire format).
 * @param {jspb.ByteSource} bytes The bytes to deserialize.
 * @return {!proto.aptos.transaction.testing1.v1.EventKey}
 */
proto.aptos.transaction.testing1.v1.EventKey.deserializeBinary = function(bytes) {
  var reader = new jspb.BinaryReader(bytes);
  var msg = new proto.aptos.transaction.testing1.v1.EventKey;
  return proto.aptos.transaction.testing1.v1.EventKey.deserializeBinaryFromReader(msg, reader);
};


/**
 * Deserializes binary data (in protobuf wire format) from the
 * given reader into the given message object.
 * @param {!proto.aptos.transaction.testing1.v1.EventKey} msg The message object to deserialize into.
 * @param {!jspb.BinaryReader} reader The BinaryReader to use.
 * @return {!proto.aptos.transaction.testing1.v1.EventKey}
 */
proto.aptos.transaction.testing1.v1.EventKey.deserializeBinaryFromReader = function(msg, reader) {
  while (reader.nextField()) {
    if (reader.isEndGroup()) {
      break;
    }
    var field = reader.getFieldNumber();
    switch (field) {
    case 1:
      var value = /** @type {number} */ (reader.readUint64());
      msg.setCreationNumber(value);
      break;
    case 2:
      var value = /** @type {string} */ (reader.readString());
      msg.setAccountAddress(value);
      break;
    default:
      reader.skipField();
      break;
    }
  }
  return msg;
};


/**
 * Serializes the message to binary data (in protobuf wire format).
 * @return {!Uint8Array}
 */
proto.aptos.transaction.testing1.v1.EventKey.prototype.serializeBinary = function() {
  var writer = new jspb.BinaryWriter();
  proto.aptos.transaction.testing1.v1.EventKey.serializeBinaryToWriter(this, writer);
  return writer.getResultBuffer();
};


/**
 * Serializes the given message to binary data (in protobuf wire
 * format), writing to the given BinaryWriter.
 * @param {!proto.aptos.transaction.testing1.v1.EventKey} message
 * @param {!jspb.BinaryWriter} writer
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.aptos.transaction.testing1.v1.EventKey.serializeBinaryToWriter = function(message, writer) {
  var f = undefined;
  f = message.getCreationNumber();
  if (f !== 0) {
    writer.writeUint64(
      1,
      f
    );
  }
  f = message.getAccountAddress();
  if (f.length > 0) {
    writer.writeString(
      2,
      f
    );
  }
};


/**
 * optional uint64 creation_number = 1;
 * @return {number}
 */
proto.aptos.transaction.testing1.v1.EventKey.prototype.getCreationNumber = function() {
  return /** @type {number} */ (jspb.Message.getFieldWithDefault(this, 1, 0));
};


/**
 * @param {number} value
 * @return {!proto.aptos.transaction.testing1.v1.EventKey} returns this
 */
proto.aptos.transaction.testing1.v1.EventKey.prototype.setCreationNumber = function(value) {
  return jspb.Message.setProto3IntField(this, 1, value);
};


/**
 * optional string account_address = 2;
 * @return {string}
 */
proto.aptos.transaction.testing1.v1.EventKey.prototype.getAccountAddress = function() {
  return /** @type {string} */ (jspb.Message.getFieldWithDefault(this, 2, ""));
};


/**
 * @param {string} value
 * @return {!proto.aptos.transaction.testing1.v1.EventKey} returns this
 */
proto.aptos.transaction.testing1.v1.EventKey.prototype.setAccountAddress = function(value) {
  return jspb.Message.setProto3StringField(this, 2, value);
};





if (jspb.Message.GENERATE_TO_OBJECT) {
/**
 * Creates an object representation of this proto.
 * Field names that are reserved in JavaScript and will be renamed to pb_name.
 * Optional fields that are not set will be set to undefined.
 * To access a reserved field use, foo.pb_<name>, eg, foo.pb_default.
 * For the list of reserved names please see:
 *     net/proto2/compiler/js/internal/generator.cc#kKeyword.
 * @param {boolean=} opt_includeInstance Deprecated. whether to include the
 *     JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @return {!Object}
 */
proto.aptos.transaction.testing1.v1.UserTransactionRequest.prototype.toObject = function(opt_includeInstance) {
  return proto.aptos.transaction.testing1.v1.UserTransactionRequest.toObject(opt_includeInstance, this);
};


/**
 * Static version of the {@see toObject} method.
 * @param {boolean|undefined} includeInstance Deprecated. Whether to include
 *     the JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @param {!proto.aptos.transaction.testing1.v1.UserTransactionRequest} msg The msg instance to transform.
 * @return {!Object}
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.aptos.transaction.testing1.v1.UserTransactionRequest.toObject = function(includeInstance, msg) {
  var f, obj = {
    sender: jspb.Message.getFieldWithDefault(msg, 1, ""),
    sequenceNumber: jspb.Message.getFieldWithDefault(msg, 2, 0),
    maxGasAmount: jspb.Message.getFieldWithDefault(msg, 3, 0),
    gasUnitPrice: jspb.Message.getFieldWithDefault(msg, 4, 0),
    expirationTimestampSecs: (f = msg.getExpirationTimestampSecs()) && aptos_util_timestamp_timestamp_pb.Timestamp.toObject(includeInstance, f),
    payload: (f = msg.getPayload()) && proto.aptos.transaction.testing1.v1.TransactionPayload.toObject(includeInstance, f),
    signature: (f = msg.getSignature()) && proto.aptos.transaction.testing1.v1.Signature.toObject(includeInstance, f)
  };

  if (includeInstance) {
    obj.$jspbMessageInstance = msg;
  }
  return obj;
};
}


/**
 * Deserializes binary data (in protobuf wire format).
 * @param {jspb.ByteSource} bytes The bytes to deserialize.
 * @return {!proto.aptos.transaction.testing1.v1.UserTransactionRequest}
 */
proto.aptos.transaction.testing1.v1.UserTransactionRequest.deserializeBinary = function(bytes) {
  var reader = new jspb.BinaryReader(bytes);
  var msg = new proto.aptos.transaction.testing1.v1.UserTransactionRequest;
  return proto.aptos.transaction.testing1.v1.UserTransactionRequest.deserializeBinaryFromReader(msg, reader);
};


/**
 * Deserializes binary data (in protobuf wire format) from the
 * given reader into the given message object.
 * @param {!proto.aptos.transaction.testing1.v1.UserTransactionRequest} msg The message object to deserialize into.
 * @param {!jspb.BinaryReader} reader The BinaryReader to use.
 * @return {!proto.aptos.transaction.testing1.v1.UserTransactionRequest}
 */
proto.aptos.transaction.testing1.v1.UserTransactionRequest.deserializeBinaryFromReader = function(msg, reader) {
  while (reader.nextField()) {
    if (reader.isEndGroup()) {
      break;
    }
    var field = reader.getFieldNumber();
    switch (field) {
    case 1:
      var value = /** @type {string} */ (reader.readString());
      msg.setSender(value);
      break;
    case 2:
      var value = /** @type {number} */ (reader.readUint64());
      msg.setSequenceNumber(value);
      break;
    case 3:
      var value = /** @type {number} */ (reader.readUint64());
      msg.setMaxGasAmount(value);
      break;
    case 4:
      var value = /** @type {number} */ (reader.readUint64());
      msg.setGasUnitPrice(value);
      break;
    case 5:
      var value = new aptos_util_timestamp_timestamp_pb.Timestamp;
      reader.readMessage(value,aptos_util_timestamp_timestamp_pb.Timestamp.deserializeBinaryFromReader);
      msg.setExpirationTimestampSecs(value);
      break;
    case 6:
      var value = new proto.aptos.transaction.testing1.v1.TransactionPayload;
      reader.readMessage(value,proto.aptos.transaction.testing1.v1.TransactionPayload.deserializeBinaryFromReader);
      msg.setPayload(value);
      break;
    case 7:
      var value = new proto.aptos.transaction.testing1.v1.Signature;
      reader.readMessage(value,proto.aptos.transaction.testing1.v1.Signature.deserializeBinaryFromReader);
      msg.setSignature(value);
      break;
    default:
      reader.skipField();
      break;
    }
  }
  return msg;
};


/**
 * Serializes the message to binary data (in protobuf wire format).
 * @return {!Uint8Array}
 */
proto.aptos.transaction.testing1.v1.UserTransactionRequest.prototype.serializeBinary = function() {
  var writer = new jspb.BinaryWriter();
  proto.aptos.transaction.testing1.v1.UserTransactionRequest.serializeBinaryToWriter(this, writer);
  return writer.getResultBuffer();
};


/**
 * Serializes the given message to binary data (in protobuf wire
 * format), writing to the given BinaryWriter.
 * @param {!proto.aptos.transaction.testing1.v1.UserTransactionRequest} message
 * @param {!jspb.BinaryWriter} writer
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.aptos.transaction.testing1.v1.UserTransactionRequest.serializeBinaryToWriter = function(message, writer) {
  var f = undefined;
  f = message.getSender();
  if (f.length > 0) {
    writer.writeString(
      1,
      f
    );
  }
  f = message.getSequenceNumber();
  if (f !== 0) {
    writer.writeUint64(
      2,
      f
    );
  }
  f = message.getMaxGasAmount();
  if (f !== 0) {
    writer.writeUint64(
      3,
      f
    );
  }
  f = message.getGasUnitPrice();
  if (f !== 0) {
    writer.writeUint64(
      4,
      f
    );
  }
  f = message.getExpirationTimestampSecs();
  if (f != null) {
    writer.writeMessage(
      5,
      f,
      aptos_util_timestamp_timestamp_pb.Timestamp.serializeBinaryToWriter
    );
  }
  f = message.getPayload();
  if (f != null) {
    writer.writeMessage(
      6,
      f,
      proto.aptos.transaction.testing1.v1.TransactionPayload.serializeBinaryToWriter
    );
  }
  f = message.getSignature();
  if (f != null) {
    writer.writeMessage(
      7,
      f,
      proto.aptos.transaction.testing1.v1.Signature.serializeBinaryToWriter
    );
  }
};


/**
 * optional string sender = 1;
 * @return {string}
 */
proto.aptos.transaction.testing1.v1.UserTransactionRequest.prototype.getSender = function() {
  return /** @type {string} */ (jspb.Message.getFieldWithDefault(this, 1, ""));
};


/**
 * @param {string} value
 * @return {!proto.aptos.transaction.testing1.v1.UserTransactionRequest} returns this
 */
proto.aptos.transaction.testing1.v1.UserTransactionRequest.prototype.setSender = function(value) {
  return jspb.Message.setProto3StringField(this, 1, value);
};


/**
 * optional uint64 sequence_number = 2;
 * @return {number}
 */
proto.aptos.transaction.testing1.v1.UserTransactionRequest.prototype.getSequenceNumber = function() {
  return /** @type {number} */ (jspb.Message.getFieldWithDefault(this, 2, 0));
};


/**
 * @param {number} value
 * @return {!proto.aptos.transaction.testing1.v1.UserTransactionRequest} returns this
 */
proto.aptos.transaction.testing1.v1.UserTransactionRequest.prototype.setSequenceNumber = function(value) {
  return jspb.Message.setProto3IntField(this, 2, value);
};


/**
 * optional uint64 max_gas_amount = 3;
 * @return {number}
 */
proto.aptos.transaction.testing1.v1.UserTransactionRequest.prototype.getMaxGasAmount = function() {
  return /** @type {number} */ (jspb.Message.getFieldWithDefault(this, 3, 0));
};


/**
 * @param {number} value
 * @return {!proto.aptos.transaction.testing1.v1.UserTransactionRequest} returns this
 */
proto.aptos.transaction.testing1.v1.UserTransactionRequest.prototype.setMaxGasAmount = function(value) {
  return jspb.Message.setProto3IntField(this, 3, value);
};


/**
 * optional uint64 gas_unit_price = 4;
 * @return {number}
 */
proto.aptos.transaction.testing1.v1.UserTransactionRequest.prototype.getGasUnitPrice = function() {
  return /** @type {number} */ (jspb.Message.getFieldWithDefault(this, 4, 0));
};


/**
 * @param {number} value
 * @return {!proto.aptos.transaction.testing1.v1.UserTransactionRequest} returns this
 */
proto.aptos.transaction.testing1.v1.UserTransactionRequest.prototype.setGasUnitPrice = function(value) {
  return jspb.Message.setProto3IntField(this, 4, value);
};


/**
 * optional aptos.util.timestamp.Timestamp expiration_timestamp_secs = 5;
 * @return {?proto.aptos.util.timestamp.Timestamp}
 */
proto.aptos.transaction.testing1.v1.UserTransactionRequest.prototype.getExpirationTimestampSecs = function() {
  return /** @type{?proto.aptos.util.timestamp.Timestamp} */ (
    jspb.Message.getWrapperField(this, aptos_util_timestamp_timestamp_pb.Timestamp, 5));
};


/**
 * @param {?proto.aptos.util.timestamp.Timestamp|undefined} value
 * @return {!proto.aptos.transaction.testing1.v1.UserTransactionRequest} returns this
*/
proto.aptos.transaction.testing1.v1.UserTransactionRequest.prototype.setExpirationTimestampSecs = function(value) {
  return jspb.Message.setWrapperField(this, 5, value);
};


/**
 * Clears the message field making it undefined.
 * @return {!proto.aptos.transaction.testing1.v1.UserTransactionRequest} returns this
 */
proto.aptos.transaction.testing1.v1.UserTransactionRequest.prototype.clearExpirationTimestampSecs = function() {
  return this.setExpirationTimestampSecs(undefined);
};


/**
 * Returns whether this field is set.
 * @return {boolean}
 */
proto.aptos.transaction.testing1.v1.UserTransactionRequest.prototype.hasExpirationTimestampSecs = function() {
  return jspb.Message.getField(this, 5) != null;
};


/**
 * optional TransactionPayload payload = 6;
 * @return {?proto.aptos.transaction.testing1.v1.TransactionPayload}
 */
proto.aptos.transaction.testing1.v1.UserTransactionRequest.prototype.getPayload = function() {
  return /** @type{?proto.aptos.transaction.testing1.v1.TransactionPayload} */ (
    jspb.Message.getWrapperField(this, proto.aptos.transaction.testing1.v1.TransactionPayload, 6));
};


/**
 * @param {?proto.aptos.transaction.testing1.v1.TransactionPayload|undefined} value
 * @return {!proto.aptos.transaction.testing1.v1.UserTransactionRequest} returns this
*/
proto.aptos.transaction.testing1.v1.UserTransactionRequest.prototype.setPayload = function(value) {
  return jspb.Message.setWrapperField(this, 6, value);
};


/**
 * Clears the message field making it undefined.
 * @return {!proto.aptos.transaction.testing1.v1.UserTransactionRequest} returns this
 */
proto.aptos.transaction.testing1.v1.UserTransactionRequest.prototype.clearPayload = function() {
  return this.setPayload(undefined);
};


/**
 * Returns whether this field is set.
 * @return {boolean}
 */
proto.aptos.transaction.testing1.v1.UserTransactionRequest.prototype.hasPayload = function() {
  return jspb.Message.getField(this, 6) != null;
};


/**
 * optional Signature signature = 7;
 * @return {?proto.aptos.transaction.testing1.v1.Signature}
 */
proto.aptos.transaction.testing1.v1.UserTransactionRequest.prototype.getSignature = function() {
  return /** @type{?proto.aptos.transaction.testing1.v1.Signature} */ (
    jspb.Message.getWrapperField(this, proto.aptos.transaction.testing1.v1.Signature, 7));
};


/**
 * @param {?proto.aptos.transaction.testing1.v1.Signature|undefined} value
 * @return {!proto.aptos.transaction.testing1.v1.UserTransactionRequest} returns this
*/
proto.aptos.transaction.testing1.v1.UserTransactionRequest.prototype.setSignature = function(value) {
  return jspb.Message.setWrapperField(this, 7, value);
};


/**
 * Clears the message field making it undefined.
 * @return {!proto.aptos.transaction.testing1.v1.UserTransactionRequest} returns this
 */
proto.aptos.transaction.testing1.v1.UserTransactionRequest.prototype.clearSignature = function() {
  return this.setSignature(undefined);
};


/**
 * Returns whether this field is set.
 * @return {boolean}
 */
proto.aptos.transaction.testing1.v1.UserTransactionRequest.prototype.hasSignature = function() {
  return jspb.Message.getField(this, 7) != null;
};



/**
 * Oneof group definitions for this message. Each group defines the field
 * numbers belonging to that group. When of these fields' value is set, all
 * other fields in the group are cleared. During deserialization, if multiple
 * fields are encountered for a group, only the last value seen will be kept.
 * @private {!Array<!Array<number>>}
 * @const
 */
proto.aptos.transaction.testing1.v1.WriteSet.oneofGroups_ = [[2,3]];

/**
 * @enum {number}
 */
proto.aptos.transaction.testing1.v1.WriteSet.WriteSetCase = {
  WRITE_SET_NOT_SET: 0,
  SCRIPT_WRITE_SET: 2,
  DIRECT_WRITE_SET: 3
};

/**
 * @return {proto.aptos.transaction.testing1.v1.WriteSet.WriteSetCase}
 */
proto.aptos.transaction.testing1.v1.WriteSet.prototype.getWriteSetCase = function() {
  return /** @type {proto.aptos.transaction.testing1.v1.WriteSet.WriteSetCase} */(jspb.Message.computeOneofCase(this, proto.aptos.transaction.testing1.v1.WriteSet.oneofGroups_[0]));
};



if (jspb.Message.GENERATE_TO_OBJECT) {
/**
 * Creates an object representation of this proto.
 * Field names that are reserved in JavaScript and will be renamed to pb_name.
 * Optional fields that are not set will be set to undefined.
 * To access a reserved field use, foo.pb_<name>, eg, foo.pb_default.
 * For the list of reserved names please see:
 *     net/proto2/compiler/js/internal/generator.cc#kKeyword.
 * @param {boolean=} opt_includeInstance Deprecated. whether to include the
 *     JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @return {!Object}
 */
proto.aptos.transaction.testing1.v1.WriteSet.prototype.toObject = function(opt_includeInstance) {
  return proto.aptos.transaction.testing1.v1.WriteSet.toObject(opt_includeInstance, this);
};


/**
 * Static version of the {@see toObject} method.
 * @param {boolean|undefined} includeInstance Deprecated. Whether to include
 *     the JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @param {!proto.aptos.transaction.testing1.v1.WriteSet} msg The msg instance to transform.
 * @return {!Object}
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.aptos.transaction.testing1.v1.WriteSet.toObject = function(includeInstance, msg) {
  var f, obj = {
    writeSetType: jspb.Message.getFieldWithDefault(msg, 1, 0),
    scriptWriteSet: (f = msg.getScriptWriteSet()) && proto.aptos.transaction.testing1.v1.ScriptWriteSet.toObject(includeInstance, f),
    directWriteSet: (f = msg.getDirectWriteSet()) && proto.aptos.transaction.testing1.v1.DirectWriteSet.toObject(includeInstance, f)
  };

  if (includeInstance) {
    obj.$jspbMessageInstance = msg;
  }
  return obj;
};
}


/**
 * Deserializes binary data (in protobuf wire format).
 * @param {jspb.ByteSource} bytes The bytes to deserialize.
 * @return {!proto.aptos.transaction.testing1.v1.WriteSet}
 */
proto.aptos.transaction.testing1.v1.WriteSet.deserializeBinary = function(bytes) {
  var reader = new jspb.BinaryReader(bytes);
  var msg = new proto.aptos.transaction.testing1.v1.WriteSet;
  return proto.aptos.transaction.testing1.v1.WriteSet.deserializeBinaryFromReader(msg, reader);
};


/**
 * Deserializes binary data (in protobuf wire format) from the
 * given reader into the given message object.
 * @param {!proto.aptos.transaction.testing1.v1.WriteSet} msg The message object to deserialize into.
 * @param {!jspb.BinaryReader} reader The BinaryReader to use.
 * @return {!proto.aptos.transaction.testing1.v1.WriteSet}
 */
proto.aptos.transaction.testing1.v1.WriteSet.deserializeBinaryFromReader = function(msg, reader) {
  while (reader.nextField()) {
    if (reader.isEndGroup()) {
      break;
    }
    var field = reader.getFieldNumber();
    switch (field) {
    case 1:
      var value = /** @type {!proto.aptos.transaction.testing1.v1.WriteSet.WriteSetType} */ (reader.readEnum());
      msg.setWriteSetType(value);
      break;
    case 2:
      var value = new proto.aptos.transaction.testing1.v1.ScriptWriteSet;
      reader.readMessage(value,proto.aptos.transaction.testing1.v1.ScriptWriteSet.deserializeBinaryFromReader);
      msg.setScriptWriteSet(value);
      break;
    case 3:
      var value = new proto.aptos.transaction.testing1.v1.DirectWriteSet;
      reader.readMessage(value,proto.aptos.transaction.testing1.v1.DirectWriteSet.deserializeBinaryFromReader);
      msg.setDirectWriteSet(value);
      break;
    default:
      reader.skipField();
      break;
    }
  }
  return msg;
};


/**
 * Serializes the message to binary data (in protobuf wire format).
 * @return {!Uint8Array}
 */
proto.aptos.transaction.testing1.v1.WriteSet.prototype.serializeBinary = function() {
  var writer = new jspb.BinaryWriter();
  proto.aptos.transaction.testing1.v1.WriteSet.serializeBinaryToWriter(this, writer);
  return writer.getResultBuffer();
};


/**
 * Serializes the given message to binary data (in protobuf wire
 * format), writing to the given BinaryWriter.
 * @param {!proto.aptos.transaction.testing1.v1.WriteSet} message
 * @param {!jspb.BinaryWriter} writer
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.aptos.transaction.testing1.v1.WriteSet.serializeBinaryToWriter = function(message, writer) {
  var f = undefined;
  f = message.getWriteSetType();
  if (f !== 0.0) {
    writer.writeEnum(
      1,
      f
    );
  }
  f = message.getScriptWriteSet();
  if (f != null) {
    writer.writeMessage(
      2,
      f,
      proto.aptos.transaction.testing1.v1.ScriptWriteSet.serializeBinaryToWriter
    );
  }
  f = message.getDirectWriteSet();
  if (f != null) {
    writer.writeMessage(
      3,
      f,
      proto.aptos.transaction.testing1.v1.DirectWriteSet.serializeBinaryToWriter
    );
  }
};


/**
 * @enum {number}
 */
proto.aptos.transaction.testing1.v1.WriteSet.WriteSetType = {
  WRITE_SET_TYPE_UNSPECIFIED: 0,
  WRITE_SET_TYPE_SCRIPT_WRITE_SET: 1,
  WRITE_SET_TYPE_DIRECT_WRITE_SET: 2
};

/**
 * optional WriteSetType write_set_type = 1;
 * @return {!proto.aptos.transaction.testing1.v1.WriteSet.WriteSetType}
 */
proto.aptos.transaction.testing1.v1.WriteSet.prototype.getWriteSetType = function() {
  return /** @type {!proto.aptos.transaction.testing1.v1.WriteSet.WriteSetType} */ (jspb.Message.getFieldWithDefault(this, 1, 0));
};


/**
 * @param {!proto.aptos.transaction.testing1.v1.WriteSet.WriteSetType} value
 * @return {!proto.aptos.transaction.testing1.v1.WriteSet} returns this
 */
proto.aptos.transaction.testing1.v1.WriteSet.prototype.setWriteSetType = function(value) {
  return jspb.Message.setProto3EnumField(this, 1, value);
};


/**
 * optional ScriptWriteSet script_write_set = 2;
 * @return {?proto.aptos.transaction.testing1.v1.ScriptWriteSet}
 */
proto.aptos.transaction.testing1.v1.WriteSet.prototype.getScriptWriteSet = function() {
  return /** @type{?proto.aptos.transaction.testing1.v1.ScriptWriteSet} */ (
    jspb.Message.getWrapperField(this, proto.aptos.transaction.testing1.v1.ScriptWriteSet, 2));
};


/**
 * @param {?proto.aptos.transaction.testing1.v1.ScriptWriteSet|undefined} value
 * @return {!proto.aptos.transaction.testing1.v1.WriteSet} returns this
*/
proto.aptos.transaction.testing1.v1.WriteSet.prototype.setScriptWriteSet = function(value) {
  return jspb.Message.setOneofWrapperField(this, 2, proto.aptos.transaction.testing1.v1.WriteSet.oneofGroups_[0], value);
};


/**
 * Clears the message field making it undefined.
 * @return {!proto.aptos.transaction.testing1.v1.WriteSet} returns this
 */
proto.aptos.transaction.testing1.v1.WriteSet.prototype.clearScriptWriteSet = function() {
  return this.setScriptWriteSet(undefined);
};


/**
 * Returns whether this field is set.
 * @return {boolean}
 */
proto.aptos.transaction.testing1.v1.WriteSet.prototype.hasScriptWriteSet = function() {
  return jspb.Message.getField(this, 2) != null;
};


/**
 * optional DirectWriteSet direct_write_set = 3;
 * @return {?proto.aptos.transaction.testing1.v1.DirectWriteSet}
 */
proto.aptos.transaction.testing1.v1.WriteSet.prototype.getDirectWriteSet = function() {
  return /** @type{?proto.aptos.transaction.testing1.v1.DirectWriteSet} */ (
    jspb.Message.getWrapperField(this, proto.aptos.transaction.testing1.v1.DirectWriteSet, 3));
};


/**
 * @param {?proto.aptos.transaction.testing1.v1.DirectWriteSet|undefined} value
 * @return {!proto.aptos.transaction.testing1.v1.WriteSet} returns this
*/
proto.aptos.transaction.testing1.v1.WriteSet.prototype.setDirectWriteSet = function(value) {
  return jspb.Message.setOneofWrapperField(this, 3, proto.aptos.transaction.testing1.v1.WriteSet.oneofGroups_[0], value);
};


/**
 * Clears the message field making it undefined.
 * @return {!proto.aptos.transaction.testing1.v1.WriteSet} returns this
 */
proto.aptos.transaction.testing1.v1.WriteSet.prototype.clearDirectWriteSet = function() {
  return this.setDirectWriteSet(undefined);
};


/**
 * Returns whether this field is set.
 * @return {boolean}
 */
proto.aptos.transaction.testing1.v1.WriteSet.prototype.hasDirectWriteSet = function() {
  return jspb.Message.getField(this, 3) != null;
};





if (jspb.Message.GENERATE_TO_OBJECT) {
/**
 * Creates an object representation of this proto.
 * Field names that are reserved in JavaScript and will be renamed to pb_name.
 * Optional fields that are not set will be set to undefined.
 * To access a reserved field use, foo.pb_<name>, eg, foo.pb_default.
 * For the list of reserved names please see:
 *     net/proto2/compiler/js/internal/generator.cc#kKeyword.
 * @param {boolean=} opt_includeInstance Deprecated. whether to include the
 *     JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @return {!Object}
 */
proto.aptos.transaction.testing1.v1.ScriptWriteSet.prototype.toObject = function(opt_includeInstance) {
  return proto.aptos.transaction.testing1.v1.ScriptWriteSet.toObject(opt_includeInstance, this);
};


/**
 * Static version of the {@see toObject} method.
 * @param {boolean|undefined} includeInstance Deprecated. Whether to include
 *     the JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @param {!proto.aptos.transaction.testing1.v1.ScriptWriteSet} msg The msg instance to transform.
 * @return {!Object}
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.aptos.transaction.testing1.v1.ScriptWriteSet.toObject = function(includeInstance, msg) {
  var f, obj = {
    executeAs: jspb.Message.getFieldWithDefault(msg, 1, ""),
    script: (f = msg.getScript()) && proto.aptos.transaction.testing1.v1.ScriptPayload.toObject(includeInstance, f)
  };

  if (includeInstance) {
    obj.$jspbMessageInstance = msg;
  }
  return obj;
};
}


/**
 * Deserializes binary data (in protobuf wire format).
 * @param {jspb.ByteSource} bytes The bytes to deserialize.
 * @return {!proto.aptos.transaction.testing1.v1.ScriptWriteSet}
 */
proto.aptos.transaction.testing1.v1.ScriptWriteSet.deserializeBinary = function(bytes) {
  var reader = new jspb.BinaryReader(bytes);
  var msg = new proto.aptos.transaction.testing1.v1.ScriptWriteSet;
  return proto.aptos.transaction.testing1.v1.ScriptWriteSet.deserializeBinaryFromReader(msg, reader);
};


/**
 * Deserializes binary data (in protobuf wire format) from the
 * given reader into the given message object.
 * @param {!proto.aptos.transaction.testing1.v1.ScriptWriteSet} msg The message object to deserialize into.
 * @param {!jspb.BinaryReader} reader The BinaryReader to use.
 * @return {!proto.aptos.transaction.testing1.v1.ScriptWriteSet}
 */
proto.aptos.transaction.testing1.v1.ScriptWriteSet.deserializeBinaryFromReader = function(msg, reader) {
  while (reader.nextField()) {
    if (reader.isEndGroup()) {
      break;
    }
    var field = reader.getFieldNumber();
    switch (field) {
    case 1:
      var value = /** @type {string} */ (reader.readString());
      msg.setExecuteAs(value);
      break;
    case 2:
      var value = new proto.aptos.transaction.testing1.v1.ScriptPayload;
      reader.readMessage(value,proto.aptos.transaction.testing1.v1.ScriptPayload.deserializeBinaryFromReader);
      msg.setScript(value);
      break;
    default:
      reader.skipField();
      break;
    }
  }
  return msg;
};


/**
 * Serializes the message to binary data (in protobuf wire format).
 * @return {!Uint8Array}
 */
proto.aptos.transaction.testing1.v1.ScriptWriteSet.prototype.serializeBinary = function() {
  var writer = new jspb.BinaryWriter();
  proto.aptos.transaction.testing1.v1.ScriptWriteSet.serializeBinaryToWriter(this, writer);
  return writer.getResultBuffer();
};


/**
 * Serializes the given message to binary data (in protobuf wire
 * format), writing to the given BinaryWriter.
 * @param {!proto.aptos.transaction.testing1.v1.ScriptWriteSet} message
 * @param {!jspb.BinaryWriter} writer
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.aptos.transaction.testing1.v1.ScriptWriteSet.serializeBinaryToWriter = function(message, writer) {
  var f = undefined;
  f = message.getExecuteAs();
  if (f.length > 0) {
    writer.writeString(
      1,
      f
    );
  }
  f = message.getScript();
  if (f != null) {
    writer.writeMessage(
      2,
      f,
      proto.aptos.transaction.testing1.v1.ScriptPayload.serializeBinaryToWriter
    );
  }
};


/**
 * optional string execute_as = 1;
 * @return {string}
 */
proto.aptos.transaction.testing1.v1.ScriptWriteSet.prototype.getExecuteAs = function() {
  return /** @type {string} */ (jspb.Message.getFieldWithDefault(this, 1, ""));
};


/**
 * @param {string} value
 * @return {!proto.aptos.transaction.testing1.v1.ScriptWriteSet} returns this
 */
proto.aptos.transaction.testing1.v1.ScriptWriteSet.prototype.setExecuteAs = function(value) {
  return jspb.Message.setProto3StringField(this, 1, value);
};


/**
 * optional ScriptPayload script = 2;
 * @return {?proto.aptos.transaction.testing1.v1.ScriptPayload}
 */
proto.aptos.transaction.testing1.v1.ScriptWriteSet.prototype.getScript = function() {
  return /** @type{?proto.aptos.transaction.testing1.v1.ScriptPayload} */ (
    jspb.Message.getWrapperField(this, proto.aptos.transaction.testing1.v1.ScriptPayload, 2));
};


/**
 * @param {?proto.aptos.transaction.testing1.v1.ScriptPayload|undefined} value
 * @return {!proto.aptos.transaction.testing1.v1.ScriptWriteSet} returns this
*/
proto.aptos.transaction.testing1.v1.ScriptWriteSet.prototype.setScript = function(value) {
  return jspb.Message.setWrapperField(this, 2, value);
};


/**
 * Clears the message field making it undefined.
 * @return {!proto.aptos.transaction.testing1.v1.ScriptWriteSet} returns this
 */
proto.aptos.transaction.testing1.v1.ScriptWriteSet.prototype.clearScript = function() {
  return this.setScript(undefined);
};


/**
 * Returns whether this field is set.
 * @return {boolean}
 */
proto.aptos.transaction.testing1.v1.ScriptWriteSet.prototype.hasScript = function() {
  return jspb.Message.getField(this, 2) != null;
};



/**
 * List of repeated fields within this message type.
 * @private {!Array<number>}
 * @const
 */
proto.aptos.transaction.testing1.v1.DirectWriteSet.repeatedFields_ = [1,2];



if (jspb.Message.GENERATE_TO_OBJECT) {
/**
 * Creates an object representation of this proto.
 * Field names that are reserved in JavaScript and will be renamed to pb_name.
 * Optional fields that are not set will be set to undefined.
 * To access a reserved field use, foo.pb_<name>, eg, foo.pb_default.
 * For the list of reserved names please see:
 *     net/proto2/compiler/js/internal/generator.cc#kKeyword.
 * @param {boolean=} opt_includeInstance Deprecated. whether to include the
 *     JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @return {!Object}
 */
proto.aptos.transaction.testing1.v1.DirectWriteSet.prototype.toObject = function(opt_includeInstance) {
  return proto.aptos.transaction.testing1.v1.DirectWriteSet.toObject(opt_includeInstance, this);
};


/**
 * Static version of the {@see toObject} method.
 * @param {boolean|undefined} includeInstance Deprecated. Whether to include
 *     the JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @param {!proto.aptos.transaction.testing1.v1.DirectWriteSet} msg The msg instance to transform.
 * @return {!Object}
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.aptos.transaction.testing1.v1.DirectWriteSet.toObject = function(includeInstance, msg) {
  var f, obj = {
    writeSetChangeList: jspb.Message.toObjectList(msg.getWriteSetChangeList(),
    proto.aptos.transaction.testing1.v1.WriteSetChange.toObject, includeInstance),
    eventsList: jspb.Message.toObjectList(msg.getEventsList(),
    proto.aptos.transaction.testing1.v1.Event.toObject, includeInstance)
  };

  if (includeInstance) {
    obj.$jspbMessageInstance = msg;
  }
  return obj;
};
}


/**
 * Deserializes binary data (in protobuf wire format).
 * @param {jspb.ByteSource} bytes The bytes to deserialize.
 * @return {!proto.aptos.transaction.testing1.v1.DirectWriteSet}
 */
proto.aptos.transaction.testing1.v1.DirectWriteSet.deserializeBinary = function(bytes) {
  var reader = new jspb.BinaryReader(bytes);
  var msg = new proto.aptos.transaction.testing1.v1.DirectWriteSet;
  return proto.aptos.transaction.testing1.v1.DirectWriteSet.deserializeBinaryFromReader(msg, reader);
};


/**
 * Deserializes binary data (in protobuf wire format) from the
 * given reader into the given message object.
 * @param {!proto.aptos.transaction.testing1.v1.DirectWriteSet} msg The message object to deserialize into.
 * @param {!jspb.BinaryReader} reader The BinaryReader to use.
 * @return {!proto.aptos.transaction.testing1.v1.DirectWriteSet}
 */
proto.aptos.transaction.testing1.v1.DirectWriteSet.deserializeBinaryFromReader = function(msg, reader) {
  while (reader.nextField()) {
    if (reader.isEndGroup()) {
      break;
    }
    var field = reader.getFieldNumber();
    switch (field) {
    case 1:
      var value = new proto.aptos.transaction.testing1.v1.WriteSetChange;
      reader.readMessage(value,proto.aptos.transaction.testing1.v1.WriteSetChange.deserializeBinaryFromReader);
      msg.addWriteSetChange(value);
      break;
    case 2:
      var value = new proto.aptos.transaction.testing1.v1.Event;
      reader.readMessage(value,proto.aptos.transaction.testing1.v1.Event.deserializeBinaryFromReader);
      msg.addEvents(value);
      break;
    default:
      reader.skipField();
      break;
    }
  }
  return msg;
};


/**
 * Serializes the message to binary data (in protobuf wire format).
 * @return {!Uint8Array}
 */
proto.aptos.transaction.testing1.v1.DirectWriteSet.prototype.serializeBinary = function() {
  var writer = new jspb.BinaryWriter();
  proto.aptos.transaction.testing1.v1.DirectWriteSet.serializeBinaryToWriter(this, writer);
  return writer.getResultBuffer();
};


/**
 * Serializes the given message to binary data (in protobuf wire
 * format), writing to the given BinaryWriter.
 * @param {!proto.aptos.transaction.testing1.v1.DirectWriteSet} message
 * @param {!jspb.BinaryWriter} writer
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.aptos.transaction.testing1.v1.DirectWriteSet.serializeBinaryToWriter = function(message, writer) {
  var f = undefined;
  f = message.getWriteSetChangeList();
  if (f.length > 0) {
    writer.writeRepeatedMessage(
      1,
      f,
      proto.aptos.transaction.testing1.v1.WriteSetChange.serializeBinaryToWriter
    );
  }
  f = message.getEventsList();
  if (f.length > 0) {
    writer.writeRepeatedMessage(
      2,
      f,
      proto.aptos.transaction.testing1.v1.Event.serializeBinaryToWriter
    );
  }
};


/**
 * repeated WriteSetChange write_set_change = 1;
 * @return {!Array<!proto.aptos.transaction.testing1.v1.WriteSetChange>}
 */
proto.aptos.transaction.testing1.v1.DirectWriteSet.prototype.getWriteSetChangeList = function() {
  return /** @type{!Array<!proto.aptos.transaction.testing1.v1.WriteSetChange>} */ (
    jspb.Message.getRepeatedWrapperField(this, proto.aptos.transaction.testing1.v1.WriteSetChange, 1));
};


/**
 * @param {!Array<!proto.aptos.transaction.testing1.v1.WriteSetChange>} value
 * @return {!proto.aptos.transaction.testing1.v1.DirectWriteSet} returns this
*/
proto.aptos.transaction.testing1.v1.DirectWriteSet.prototype.setWriteSetChangeList = function(value) {
  return jspb.Message.setRepeatedWrapperField(this, 1, value);
};


/**
 * @param {!proto.aptos.transaction.testing1.v1.WriteSetChange=} opt_value
 * @param {number=} opt_index
 * @return {!proto.aptos.transaction.testing1.v1.WriteSetChange}
 */
proto.aptos.transaction.testing1.v1.DirectWriteSet.prototype.addWriteSetChange = function(opt_value, opt_index) {
  return jspb.Message.addToRepeatedWrapperField(this, 1, opt_value, proto.aptos.transaction.testing1.v1.WriteSetChange, opt_index);
};


/**
 * Clears the list making it empty but non-null.
 * @return {!proto.aptos.transaction.testing1.v1.DirectWriteSet} returns this
 */
proto.aptos.transaction.testing1.v1.DirectWriteSet.prototype.clearWriteSetChangeList = function() {
  return this.setWriteSetChangeList([]);
};


/**
 * repeated Event events = 2;
 * @return {!Array<!proto.aptos.transaction.testing1.v1.Event>}
 */
proto.aptos.transaction.testing1.v1.DirectWriteSet.prototype.getEventsList = function() {
  return /** @type{!Array<!proto.aptos.transaction.testing1.v1.Event>} */ (
    jspb.Message.getRepeatedWrapperField(this, proto.aptos.transaction.testing1.v1.Event, 2));
};


/**
 * @param {!Array<!proto.aptos.transaction.testing1.v1.Event>} value
 * @return {!proto.aptos.transaction.testing1.v1.DirectWriteSet} returns this
*/
proto.aptos.transaction.testing1.v1.DirectWriteSet.prototype.setEventsList = function(value) {
  return jspb.Message.setRepeatedWrapperField(this, 2, value);
};


/**
 * @param {!proto.aptos.transaction.testing1.v1.Event=} opt_value
 * @param {number=} opt_index
 * @return {!proto.aptos.transaction.testing1.v1.Event}
 */
proto.aptos.transaction.testing1.v1.DirectWriteSet.prototype.addEvents = function(opt_value, opt_index) {
  return jspb.Message.addToRepeatedWrapperField(this, 2, opt_value, proto.aptos.transaction.testing1.v1.Event, opt_index);
};


/**
 * Clears the list making it empty but non-null.
 * @return {!proto.aptos.transaction.testing1.v1.DirectWriteSet} returns this
 */
proto.aptos.transaction.testing1.v1.DirectWriteSet.prototype.clearEventsList = function() {
  return this.setEventsList([]);
};



/**
 * Oneof group definitions for this message. Each group defines the field
 * numbers belonging to that group. When of these fields' value is set, all
 * other fields in the group are cleared. During deserialization, if multiple
 * fields are encountered for a group, only the last value seen will be kept.
 * @private {!Array<!Array<number>>}
 * @const
 */
proto.aptos.transaction.testing1.v1.WriteSetChange.oneofGroups_ = [[2,3,4,5,6,7]];

/**
 * @enum {number}
 */
proto.aptos.transaction.testing1.v1.WriteSetChange.ChangeCase = {
  CHANGE_NOT_SET: 0,
  DELETE_MODULE: 2,
  DELETE_RESOURCE: 3,
  DELETE_TABLE_ITEM: 4,
  WRITE_MODULE: 5,
  WRITE_RESOURCE: 6,
  WRITE_TABLE_ITEM: 7
};

/**
 * @return {proto.aptos.transaction.testing1.v1.WriteSetChange.ChangeCase}
 */
proto.aptos.transaction.testing1.v1.WriteSetChange.prototype.getChangeCase = function() {
  return /** @type {proto.aptos.transaction.testing1.v1.WriteSetChange.ChangeCase} */(jspb.Message.computeOneofCase(this, proto.aptos.transaction.testing1.v1.WriteSetChange.oneofGroups_[0]));
};



if (jspb.Message.GENERATE_TO_OBJECT) {
/**
 * Creates an object representation of this proto.
 * Field names that are reserved in JavaScript and will be renamed to pb_name.
 * Optional fields that are not set will be set to undefined.
 * To access a reserved field use, foo.pb_<name>, eg, foo.pb_default.
 * For the list of reserved names please see:
 *     net/proto2/compiler/js/internal/generator.cc#kKeyword.
 * @param {boolean=} opt_includeInstance Deprecated. whether to include the
 *     JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @return {!Object}
 */
proto.aptos.transaction.testing1.v1.WriteSetChange.prototype.toObject = function(opt_includeInstance) {
  return proto.aptos.transaction.testing1.v1.WriteSetChange.toObject(opt_includeInstance, this);
};


/**
 * Static version of the {@see toObject} method.
 * @param {boolean|undefined} includeInstance Deprecated. Whether to include
 *     the JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @param {!proto.aptos.transaction.testing1.v1.WriteSetChange} msg The msg instance to transform.
 * @return {!Object}
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.aptos.transaction.testing1.v1.WriteSetChange.toObject = function(includeInstance, msg) {
  var f, obj = {
    type: jspb.Message.getFieldWithDefault(msg, 1, 0),
    deleteModule: (f = msg.getDeleteModule()) && proto.aptos.transaction.testing1.v1.DeleteModule.toObject(includeInstance, f),
    deleteResource: (f = msg.getDeleteResource()) && proto.aptos.transaction.testing1.v1.DeleteResource.toObject(includeInstance, f),
    deleteTableItem: (f = msg.getDeleteTableItem()) && proto.aptos.transaction.testing1.v1.DeleteTableItem.toObject(includeInstance, f),
    writeModule: (f = msg.getWriteModule()) && proto.aptos.transaction.testing1.v1.WriteModule.toObject(includeInstance, f),
    writeResource: (f = msg.getWriteResource()) && proto.aptos.transaction.testing1.v1.WriteResource.toObject(includeInstance, f),
    writeTableItem: (f = msg.getWriteTableItem()) && proto.aptos.transaction.testing1.v1.WriteTableItem.toObject(includeInstance, f)
  };

  if (includeInstance) {
    obj.$jspbMessageInstance = msg;
  }
  return obj;
};
}


/**
 * Deserializes binary data (in protobuf wire format).
 * @param {jspb.ByteSource} bytes The bytes to deserialize.
 * @return {!proto.aptos.transaction.testing1.v1.WriteSetChange}
 */
proto.aptos.transaction.testing1.v1.WriteSetChange.deserializeBinary = function(bytes) {
  var reader = new jspb.BinaryReader(bytes);
  var msg = new proto.aptos.transaction.testing1.v1.WriteSetChange;
  return proto.aptos.transaction.testing1.v1.WriteSetChange.deserializeBinaryFromReader(msg, reader);
};


/**
 * Deserializes binary data (in protobuf wire format) from the
 * given reader into the given message object.
 * @param {!proto.aptos.transaction.testing1.v1.WriteSetChange} msg The message object to deserialize into.
 * @param {!jspb.BinaryReader} reader The BinaryReader to use.
 * @return {!proto.aptos.transaction.testing1.v1.WriteSetChange}
 */
proto.aptos.transaction.testing1.v1.WriteSetChange.deserializeBinaryFromReader = function(msg, reader) {
  while (reader.nextField()) {
    if (reader.isEndGroup()) {
      break;
    }
    var field = reader.getFieldNumber();
    switch (field) {
    case 1:
      var value = /** @type {!proto.aptos.transaction.testing1.v1.WriteSetChange.Type} */ (reader.readEnum());
      msg.setType(value);
      break;
    case 2:
      var value = new proto.aptos.transaction.testing1.v1.DeleteModule;
      reader.readMessage(value,proto.aptos.transaction.testing1.v1.DeleteModule.deserializeBinaryFromReader);
      msg.setDeleteModule(value);
      break;
    case 3:
      var value = new proto.aptos.transaction.testing1.v1.DeleteResource;
      reader.readMessage(value,proto.aptos.transaction.testing1.v1.DeleteResource.deserializeBinaryFromReader);
      msg.setDeleteResource(value);
      break;
    case 4:
      var value = new proto.aptos.transaction.testing1.v1.DeleteTableItem;
      reader.readMessage(value,proto.aptos.transaction.testing1.v1.DeleteTableItem.deserializeBinaryFromReader);
      msg.setDeleteTableItem(value);
      break;
    case 5:
      var value = new proto.aptos.transaction.testing1.v1.WriteModule;
      reader.readMessage(value,proto.aptos.transaction.testing1.v1.WriteModule.deserializeBinaryFromReader);
      msg.setWriteModule(value);
      break;
    case 6:
      var value = new proto.aptos.transaction.testing1.v1.WriteResource;
      reader.readMessage(value,proto.aptos.transaction.testing1.v1.WriteResource.deserializeBinaryFromReader);
      msg.setWriteResource(value);
      break;
    case 7:
      var value = new proto.aptos.transaction.testing1.v1.WriteTableItem;
      reader.readMessage(value,proto.aptos.transaction.testing1.v1.WriteTableItem.deserializeBinaryFromReader);
      msg.setWriteTableItem(value);
      break;
    default:
      reader.skipField();
      break;
    }
  }
  return msg;
};


/**
 * Serializes the message to binary data (in protobuf wire format).
 * @return {!Uint8Array}
 */
proto.aptos.transaction.testing1.v1.WriteSetChange.prototype.serializeBinary = function() {
  var writer = new jspb.BinaryWriter();
  proto.aptos.transaction.testing1.v1.WriteSetChange.serializeBinaryToWriter(this, writer);
  return writer.getResultBuffer();
};


/**
 * Serializes the given message to binary data (in protobuf wire
 * format), writing to the given BinaryWriter.
 * @param {!proto.aptos.transaction.testing1.v1.WriteSetChange} message
 * @param {!jspb.BinaryWriter} writer
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.aptos.transaction.testing1.v1.WriteSetChange.serializeBinaryToWriter = function(message, writer) {
  var f = undefined;
  f = message.getType();
  if (f !== 0.0) {
    writer.writeEnum(
      1,
      f
    );
  }
  f = message.getDeleteModule();
  if (f != null) {
    writer.writeMessage(
      2,
      f,
      proto.aptos.transaction.testing1.v1.DeleteModule.serializeBinaryToWriter
    );
  }
  f = message.getDeleteResource();
  if (f != null) {
    writer.writeMessage(
      3,
      f,
      proto.aptos.transaction.testing1.v1.DeleteResource.serializeBinaryToWriter
    );
  }
  f = message.getDeleteTableItem();
  if (f != null) {
    writer.writeMessage(
      4,
      f,
      proto.aptos.transaction.testing1.v1.DeleteTableItem.serializeBinaryToWriter
    );
  }
  f = message.getWriteModule();
  if (f != null) {
    writer.writeMessage(
      5,
      f,
      proto.aptos.transaction.testing1.v1.WriteModule.serializeBinaryToWriter
    );
  }
  f = message.getWriteResource();
  if (f != null) {
    writer.writeMessage(
      6,
      f,
      proto.aptos.transaction.testing1.v1.WriteResource.serializeBinaryToWriter
    );
  }
  f = message.getWriteTableItem();
  if (f != null) {
    writer.writeMessage(
      7,
      f,
      proto.aptos.transaction.testing1.v1.WriteTableItem.serializeBinaryToWriter
    );
  }
};


/**
 * @enum {number}
 */
proto.aptos.transaction.testing1.v1.WriteSetChange.Type = {
  TYPE_UNSPECIFIED: 0,
  TYPE_DELETE_MODULE: 1,
  TYPE_DELETE_RESOURCE: 2,
  TYPE_DELETE_TABLE_ITEM: 3,
  TYPE_WRITE_MODULE: 4,
  TYPE_WRITE_RESOURCE: 5,
  TYPE_WRITE_TABLE_ITEM: 6
};

/**
 * optional Type type = 1;
 * @return {!proto.aptos.transaction.testing1.v1.WriteSetChange.Type}
 */
proto.aptos.transaction.testing1.v1.WriteSetChange.prototype.getType = function() {
  return /** @type {!proto.aptos.transaction.testing1.v1.WriteSetChange.Type} */ (jspb.Message.getFieldWithDefault(this, 1, 0));
};


/**
 * @param {!proto.aptos.transaction.testing1.v1.WriteSetChange.Type} value
 * @return {!proto.aptos.transaction.testing1.v1.WriteSetChange} returns this
 */
proto.aptos.transaction.testing1.v1.WriteSetChange.prototype.setType = function(value) {
  return jspb.Message.setProto3EnumField(this, 1, value);
};


/**
 * optional DeleteModule delete_module = 2;
 * @return {?proto.aptos.transaction.testing1.v1.DeleteModule}
 */
proto.aptos.transaction.testing1.v1.WriteSetChange.prototype.getDeleteModule = function() {
  return /** @type{?proto.aptos.transaction.testing1.v1.DeleteModule} */ (
    jspb.Message.getWrapperField(this, proto.aptos.transaction.testing1.v1.DeleteModule, 2));
};


/**
 * @param {?proto.aptos.transaction.testing1.v1.DeleteModule|undefined} value
 * @return {!proto.aptos.transaction.testing1.v1.WriteSetChange} returns this
*/
proto.aptos.transaction.testing1.v1.WriteSetChange.prototype.setDeleteModule = function(value) {
  return jspb.Message.setOneofWrapperField(this, 2, proto.aptos.transaction.testing1.v1.WriteSetChange.oneofGroups_[0], value);
};


/**
 * Clears the message field making it undefined.
 * @return {!proto.aptos.transaction.testing1.v1.WriteSetChange} returns this
 */
proto.aptos.transaction.testing1.v1.WriteSetChange.prototype.clearDeleteModule = function() {
  return this.setDeleteModule(undefined);
};


/**
 * Returns whether this field is set.
 * @return {boolean}
 */
proto.aptos.transaction.testing1.v1.WriteSetChange.prototype.hasDeleteModule = function() {
  return jspb.Message.getField(this, 2) != null;
};


/**
 * optional DeleteResource delete_resource = 3;
 * @return {?proto.aptos.transaction.testing1.v1.DeleteResource}
 */
proto.aptos.transaction.testing1.v1.WriteSetChange.prototype.getDeleteResource = function() {
  return /** @type{?proto.aptos.transaction.testing1.v1.DeleteResource} */ (
    jspb.Message.getWrapperField(this, proto.aptos.transaction.testing1.v1.DeleteResource, 3));
};


/**
 * @param {?proto.aptos.transaction.testing1.v1.DeleteResource|undefined} value
 * @return {!proto.aptos.transaction.testing1.v1.WriteSetChange} returns this
*/
proto.aptos.transaction.testing1.v1.WriteSetChange.prototype.setDeleteResource = function(value) {
  return jspb.Message.setOneofWrapperField(this, 3, proto.aptos.transaction.testing1.v1.WriteSetChange.oneofGroups_[0], value);
};


/**
 * Clears the message field making it undefined.
 * @return {!proto.aptos.transaction.testing1.v1.WriteSetChange} returns this
 */
proto.aptos.transaction.testing1.v1.WriteSetChange.prototype.clearDeleteResource = function() {
  return this.setDeleteResource(undefined);
};


/**
 * Returns whether this field is set.
 * @return {boolean}
 */
proto.aptos.transaction.testing1.v1.WriteSetChange.prototype.hasDeleteResource = function() {
  return jspb.Message.getField(this, 3) != null;
};


/**
 * optional DeleteTableItem delete_table_item = 4;
 * @return {?proto.aptos.transaction.testing1.v1.DeleteTableItem}
 */
proto.aptos.transaction.testing1.v1.WriteSetChange.prototype.getDeleteTableItem = function() {
  return /** @type{?proto.aptos.transaction.testing1.v1.DeleteTableItem} */ (
    jspb.Message.getWrapperField(this, proto.aptos.transaction.testing1.v1.DeleteTableItem, 4));
};


/**
 * @param {?proto.aptos.transaction.testing1.v1.DeleteTableItem|undefined} value
 * @return {!proto.aptos.transaction.testing1.v1.WriteSetChange} returns this
*/
proto.aptos.transaction.testing1.v1.WriteSetChange.prototype.setDeleteTableItem = function(value) {
  return jspb.Message.setOneofWrapperField(this, 4, proto.aptos.transaction.testing1.v1.WriteSetChange.oneofGroups_[0], value);
};


/**
 * Clears the message field making it undefined.
 * @return {!proto.aptos.transaction.testing1.v1.WriteSetChange} returns this
 */
proto.aptos.transaction.testing1.v1.WriteSetChange.prototype.clearDeleteTableItem = function() {
  return this.setDeleteTableItem(undefined);
};


/**
 * Returns whether this field is set.
 * @return {boolean}
 */
proto.aptos.transaction.testing1.v1.WriteSetChange.prototype.hasDeleteTableItem = function() {
  return jspb.Message.getField(this, 4) != null;
};


/**
 * optional WriteModule write_module = 5;
 * @return {?proto.aptos.transaction.testing1.v1.WriteModule}
 */
proto.aptos.transaction.testing1.v1.WriteSetChange.prototype.getWriteModule = function() {
  return /** @type{?proto.aptos.transaction.testing1.v1.WriteModule} */ (
    jspb.Message.getWrapperField(this, proto.aptos.transaction.testing1.v1.WriteModule, 5));
};


/**
 * @param {?proto.aptos.transaction.testing1.v1.WriteModule|undefined} value
 * @return {!proto.aptos.transaction.testing1.v1.WriteSetChange} returns this
*/
proto.aptos.transaction.testing1.v1.WriteSetChange.prototype.setWriteModule = function(value) {
  return jspb.Message.setOneofWrapperField(this, 5, proto.aptos.transaction.testing1.v1.WriteSetChange.oneofGroups_[0], value);
};


/**
 * Clears the message field making it undefined.
 * @return {!proto.aptos.transaction.testing1.v1.WriteSetChange} returns this
 */
proto.aptos.transaction.testing1.v1.WriteSetChange.prototype.clearWriteModule = function() {
  return this.setWriteModule(undefined);
};


/**
 * Returns whether this field is set.
 * @return {boolean}
 */
proto.aptos.transaction.testing1.v1.WriteSetChange.prototype.hasWriteModule = function() {
  return jspb.Message.getField(this, 5) != null;
};


/**
 * optional WriteResource write_resource = 6;
 * @return {?proto.aptos.transaction.testing1.v1.WriteResource}
 */
proto.aptos.transaction.testing1.v1.WriteSetChange.prototype.getWriteResource = function() {
  return /** @type{?proto.aptos.transaction.testing1.v1.WriteResource} */ (
    jspb.Message.getWrapperField(this, proto.aptos.transaction.testing1.v1.WriteResource, 6));
};


/**
 * @param {?proto.aptos.transaction.testing1.v1.WriteResource|undefined} value
 * @return {!proto.aptos.transaction.testing1.v1.WriteSetChange} returns this
*/
proto.aptos.transaction.testing1.v1.WriteSetChange.prototype.setWriteResource = function(value) {
  return jspb.Message.setOneofWrapperField(this, 6, proto.aptos.transaction.testing1.v1.WriteSetChange.oneofGroups_[0], value);
};


/**
 * Clears the message field making it undefined.
 * @return {!proto.aptos.transaction.testing1.v1.WriteSetChange} returns this
 */
proto.aptos.transaction.testing1.v1.WriteSetChange.prototype.clearWriteResource = function() {
  return this.setWriteResource(undefined);
};


/**
 * Returns whether this field is set.
 * @return {boolean}
 */
proto.aptos.transaction.testing1.v1.WriteSetChange.prototype.hasWriteResource = function() {
  return jspb.Message.getField(this, 6) != null;
};


/**
 * optional WriteTableItem write_table_item = 7;
 * @return {?proto.aptos.transaction.testing1.v1.WriteTableItem}
 */
proto.aptos.transaction.testing1.v1.WriteSetChange.prototype.getWriteTableItem = function() {
  return /** @type{?proto.aptos.transaction.testing1.v1.WriteTableItem} */ (
    jspb.Message.getWrapperField(this, proto.aptos.transaction.testing1.v1.WriteTableItem, 7));
};


/**
 * @param {?proto.aptos.transaction.testing1.v1.WriteTableItem|undefined} value
 * @return {!proto.aptos.transaction.testing1.v1.WriteSetChange} returns this
*/
proto.aptos.transaction.testing1.v1.WriteSetChange.prototype.setWriteTableItem = function(value) {
  return jspb.Message.setOneofWrapperField(this, 7, proto.aptos.transaction.testing1.v1.WriteSetChange.oneofGroups_[0], value);
};


/**
 * Clears the message field making it undefined.
 * @return {!proto.aptos.transaction.testing1.v1.WriteSetChange} returns this
 */
proto.aptos.transaction.testing1.v1.WriteSetChange.prototype.clearWriteTableItem = function() {
  return this.setWriteTableItem(undefined);
};


/**
 * Returns whether this field is set.
 * @return {boolean}
 */
proto.aptos.transaction.testing1.v1.WriteSetChange.prototype.hasWriteTableItem = function() {
  return jspb.Message.getField(this, 7) != null;
};





if (jspb.Message.GENERATE_TO_OBJECT) {
/**
 * Creates an object representation of this proto.
 * Field names that are reserved in JavaScript and will be renamed to pb_name.
 * Optional fields that are not set will be set to undefined.
 * To access a reserved field use, foo.pb_<name>, eg, foo.pb_default.
 * For the list of reserved names please see:
 *     net/proto2/compiler/js/internal/generator.cc#kKeyword.
 * @param {boolean=} opt_includeInstance Deprecated. whether to include the
 *     JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @return {!Object}
 */
proto.aptos.transaction.testing1.v1.DeleteModule.prototype.toObject = function(opt_includeInstance) {
  return proto.aptos.transaction.testing1.v1.DeleteModule.toObject(opt_includeInstance, this);
};


/**
 * Static version of the {@see toObject} method.
 * @param {boolean|undefined} includeInstance Deprecated. Whether to include
 *     the JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @param {!proto.aptos.transaction.testing1.v1.DeleteModule} msg The msg instance to transform.
 * @return {!Object}
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.aptos.transaction.testing1.v1.DeleteModule.toObject = function(includeInstance, msg) {
  var f, obj = {
    address: jspb.Message.getFieldWithDefault(msg, 1, ""),
    stateKeyHash: msg.getStateKeyHash_asB64(),
    module: (f = msg.getModule()) && proto.aptos.transaction.testing1.v1.MoveModuleId.toObject(includeInstance, f)
  };

  if (includeInstance) {
    obj.$jspbMessageInstance = msg;
  }
  return obj;
};
}


/**
 * Deserializes binary data (in protobuf wire format).
 * @param {jspb.ByteSource} bytes The bytes to deserialize.
 * @return {!proto.aptos.transaction.testing1.v1.DeleteModule}
 */
proto.aptos.transaction.testing1.v1.DeleteModule.deserializeBinary = function(bytes) {
  var reader = new jspb.BinaryReader(bytes);
  var msg = new proto.aptos.transaction.testing1.v1.DeleteModule;
  return proto.aptos.transaction.testing1.v1.DeleteModule.deserializeBinaryFromReader(msg, reader);
};


/**
 * Deserializes binary data (in protobuf wire format) from the
 * given reader into the given message object.
 * @param {!proto.aptos.transaction.testing1.v1.DeleteModule} msg The message object to deserialize into.
 * @param {!jspb.BinaryReader} reader The BinaryReader to use.
 * @return {!proto.aptos.transaction.testing1.v1.DeleteModule}
 */
proto.aptos.transaction.testing1.v1.DeleteModule.deserializeBinaryFromReader = function(msg, reader) {
  while (reader.nextField()) {
    if (reader.isEndGroup()) {
      break;
    }
    var field = reader.getFieldNumber();
    switch (field) {
    case 1:
      var value = /** @type {string} */ (reader.readString());
      msg.setAddress(value);
      break;
    case 2:
      var value = /** @type {!Uint8Array} */ (reader.readBytes());
      msg.setStateKeyHash(value);
      break;
    case 3:
      var value = new proto.aptos.transaction.testing1.v1.MoveModuleId;
      reader.readMessage(value,proto.aptos.transaction.testing1.v1.MoveModuleId.deserializeBinaryFromReader);
      msg.setModule(value);
      break;
    default:
      reader.skipField();
      break;
    }
  }
  return msg;
};


/**
 * Serializes the message to binary data (in protobuf wire format).
 * @return {!Uint8Array}
 */
proto.aptos.transaction.testing1.v1.DeleteModule.prototype.serializeBinary = function() {
  var writer = new jspb.BinaryWriter();
  proto.aptos.transaction.testing1.v1.DeleteModule.serializeBinaryToWriter(this, writer);
  return writer.getResultBuffer();
};


/**
 * Serializes the given message to binary data (in protobuf wire
 * format), writing to the given BinaryWriter.
 * @param {!proto.aptos.transaction.testing1.v1.DeleteModule} message
 * @param {!jspb.BinaryWriter} writer
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.aptos.transaction.testing1.v1.DeleteModule.serializeBinaryToWriter = function(message, writer) {
  var f = undefined;
  f = message.getAddress();
  if (f.length > 0) {
    writer.writeString(
      1,
      f
    );
  }
  f = message.getStateKeyHash_asU8();
  if (f.length > 0) {
    writer.writeBytes(
      2,
      f
    );
  }
  f = message.getModule();
  if (f != null) {
    writer.writeMessage(
      3,
      f,
      proto.aptos.transaction.testing1.v1.MoveModuleId.serializeBinaryToWriter
    );
  }
};


/**
 * optional string address = 1;
 * @return {string}
 */
proto.aptos.transaction.testing1.v1.DeleteModule.prototype.getAddress = function() {
  return /** @type {string} */ (jspb.Message.getFieldWithDefault(this, 1, ""));
};


/**
 * @param {string} value
 * @return {!proto.aptos.transaction.testing1.v1.DeleteModule} returns this
 */
proto.aptos.transaction.testing1.v1.DeleteModule.prototype.setAddress = function(value) {
  return jspb.Message.setProto3StringField(this, 1, value);
};


/**
 * optional bytes state_key_hash = 2;
 * @return {!(string|Uint8Array)}
 */
proto.aptos.transaction.testing1.v1.DeleteModule.prototype.getStateKeyHash = function() {
  return /** @type {!(string|Uint8Array)} */ (jspb.Message.getFieldWithDefault(this, 2, ""));
};


/**
 * optional bytes state_key_hash = 2;
 * This is a type-conversion wrapper around `getStateKeyHash()`
 * @return {string}
 */
proto.aptos.transaction.testing1.v1.DeleteModule.prototype.getStateKeyHash_asB64 = function() {
  return /** @type {string} */ (jspb.Message.bytesAsB64(
      this.getStateKeyHash()));
};


/**
 * optional bytes state_key_hash = 2;
 * Note that Uint8Array is not supported on all browsers.
 * @see http://caniuse.com/Uint8Array
 * This is a type-conversion wrapper around `getStateKeyHash()`
 * @return {!Uint8Array}
 */
proto.aptos.transaction.testing1.v1.DeleteModule.prototype.getStateKeyHash_asU8 = function() {
  return /** @type {!Uint8Array} */ (jspb.Message.bytesAsU8(
      this.getStateKeyHash()));
};


/**
 * @param {!(string|Uint8Array)} value
 * @return {!proto.aptos.transaction.testing1.v1.DeleteModule} returns this
 */
proto.aptos.transaction.testing1.v1.DeleteModule.prototype.setStateKeyHash = function(value) {
  return jspb.Message.setProto3BytesField(this, 2, value);
};


/**
 * optional MoveModuleId module = 3;
 * @return {?proto.aptos.transaction.testing1.v1.MoveModuleId}
 */
proto.aptos.transaction.testing1.v1.DeleteModule.prototype.getModule = function() {
  return /** @type{?proto.aptos.transaction.testing1.v1.MoveModuleId} */ (
    jspb.Message.getWrapperField(this, proto.aptos.transaction.testing1.v1.MoveModuleId, 3));
};


/**
 * @param {?proto.aptos.transaction.testing1.v1.MoveModuleId|undefined} value
 * @return {!proto.aptos.transaction.testing1.v1.DeleteModule} returns this
*/
proto.aptos.transaction.testing1.v1.DeleteModule.prototype.setModule = function(value) {
  return jspb.Message.setWrapperField(this, 3, value);
};


/**
 * Clears the message field making it undefined.
 * @return {!proto.aptos.transaction.testing1.v1.DeleteModule} returns this
 */
proto.aptos.transaction.testing1.v1.DeleteModule.prototype.clearModule = function() {
  return this.setModule(undefined);
};


/**
 * Returns whether this field is set.
 * @return {boolean}
 */
proto.aptos.transaction.testing1.v1.DeleteModule.prototype.hasModule = function() {
  return jspb.Message.getField(this, 3) != null;
};





if (jspb.Message.GENERATE_TO_OBJECT) {
/**
 * Creates an object representation of this proto.
 * Field names that are reserved in JavaScript and will be renamed to pb_name.
 * Optional fields that are not set will be set to undefined.
 * To access a reserved field use, foo.pb_<name>, eg, foo.pb_default.
 * For the list of reserved names please see:
 *     net/proto2/compiler/js/internal/generator.cc#kKeyword.
 * @param {boolean=} opt_includeInstance Deprecated. whether to include the
 *     JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @return {!Object}
 */
proto.aptos.transaction.testing1.v1.DeleteResource.prototype.toObject = function(opt_includeInstance) {
  return proto.aptos.transaction.testing1.v1.DeleteResource.toObject(opt_includeInstance, this);
};


/**
 * Static version of the {@see toObject} method.
 * @param {boolean|undefined} includeInstance Deprecated. Whether to include
 *     the JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @param {!proto.aptos.transaction.testing1.v1.DeleteResource} msg The msg instance to transform.
 * @return {!Object}
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.aptos.transaction.testing1.v1.DeleteResource.toObject = function(includeInstance, msg) {
  var f, obj = {
    address: jspb.Message.getFieldWithDefault(msg, 1, ""),
    stateKeyHash: msg.getStateKeyHash_asB64(),
    type: (f = msg.getType()) && proto.aptos.transaction.testing1.v1.MoveStructTag.toObject(includeInstance, f),
    typeStr: jspb.Message.getFieldWithDefault(msg, 4, "")
  };

  if (includeInstance) {
    obj.$jspbMessageInstance = msg;
  }
  return obj;
};
}


/**
 * Deserializes binary data (in protobuf wire format).
 * @param {jspb.ByteSource} bytes The bytes to deserialize.
 * @return {!proto.aptos.transaction.testing1.v1.DeleteResource}
 */
proto.aptos.transaction.testing1.v1.DeleteResource.deserializeBinary = function(bytes) {
  var reader = new jspb.BinaryReader(bytes);
  var msg = new proto.aptos.transaction.testing1.v1.DeleteResource;
  return proto.aptos.transaction.testing1.v1.DeleteResource.deserializeBinaryFromReader(msg, reader);
};


/**
 * Deserializes binary data (in protobuf wire format) from the
 * given reader into the given message object.
 * @param {!proto.aptos.transaction.testing1.v1.DeleteResource} msg The message object to deserialize into.
 * @param {!jspb.BinaryReader} reader The BinaryReader to use.
 * @return {!proto.aptos.transaction.testing1.v1.DeleteResource}
 */
proto.aptos.transaction.testing1.v1.DeleteResource.deserializeBinaryFromReader = function(msg, reader) {
  while (reader.nextField()) {
    if (reader.isEndGroup()) {
      break;
    }
    var field = reader.getFieldNumber();
    switch (field) {
    case 1:
      var value = /** @type {string} */ (reader.readString());
      msg.setAddress(value);
      break;
    case 2:
      var value = /** @type {!Uint8Array} */ (reader.readBytes());
      msg.setStateKeyHash(value);
      break;
    case 3:
      var value = new proto.aptos.transaction.testing1.v1.MoveStructTag;
      reader.readMessage(value,proto.aptos.transaction.testing1.v1.MoveStructTag.deserializeBinaryFromReader);
      msg.setType(value);
      break;
    case 4:
      var value = /** @type {string} */ (reader.readString());
      msg.setTypeStr(value);
      break;
    default:
      reader.skipField();
      break;
    }
  }
  return msg;
};


/**
 * Serializes the message to binary data (in protobuf wire format).
 * @return {!Uint8Array}
 */
proto.aptos.transaction.testing1.v1.DeleteResource.prototype.serializeBinary = function() {
  var writer = new jspb.BinaryWriter();
  proto.aptos.transaction.testing1.v1.DeleteResource.serializeBinaryToWriter(this, writer);
  return writer.getResultBuffer();
};


/**
 * Serializes the given message to binary data (in protobuf wire
 * format), writing to the given BinaryWriter.
 * @param {!proto.aptos.transaction.testing1.v1.DeleteResource} message
 * @param {!jspb.BinaryWriter} writer
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.aptos.transaction.testing1.v1.DeleteResource.serializeBinaryToWriter = function(message, writer) {
  var f = undefined;
  f = message.getAddress();
  if (f.length > 0) {
    writer.writeString(
      1,
      f
    );
  }
  f = message.getStateKeyHash_asU8();
  if (f.length > 0) {
    writer.writeBytes(
      2,
      f
    );
  }
  f = message.getType();
  if (f != null) {
    writer.writeMessage(
      3,
      f,
      proto.aptos.transaction.testing1.v1.MoveStructTag.serializeBinaryToWriter
    );
  }
  f = message.getTypeStr();
  if (f.length > 0) {
    writer.writeString(
      4,
      f
    );
  }
};


/**
 * optional string address = 1;
 * @return {string}
 */
proto.aptos.transaction.testing1.v1.DeleteResource.prototype.getAddress = function() {
  return /** @type {string} */ (jspb.Message.getFieldWithDefault(this, 1, ""));
};


/**
 * @param {string} value
 * @return {!proto.aptos.transaction.testing1.v1.DeleteResource} returns this
 */
proto.aptos.transaction.testing1.v1.DeleteResource.prototype.setAddress = function(value) {
  return jspb.Message.setProto3StringField(this, 1, value);
};


/**
 * optional bytes state_key_hash = 2;
 * @return {!(string|Uint8Array)}
 */
proto.aptos.transaction.testing1.v1.DeleteResource.prototype.getStateKeyHash = function() {
  return /** @type {!(string|Uint8Array)} */ (jspb.Message.getFieldWithDefault(this, 2, ""));
};


/**
 * optional bytes state_key_hash = 2;
 * This is a type-conversion wrapper around `getStateKeyHash()`
 * @return {string}
 */
proto.aptos.transaction.testing1.v1.DeleteResource.prototype.getStateKeyHash_asB64 = function() {
  return /** @type {string} */ (jspb.Message.bytesAsB64(
      this.getStateKeyHash()));
};


/**
 * optional bytes state_key_hash = 2;
 * Note that Uint8Array is not supported on all browsers.
 * @see http://caniuse.com/Uint8Array
 * This is a type-conversion wrapper around `getStateKeyHash()`
 * @return {!Uint8Array}
 */
proto.aptos.transaction.testing1.v1.DeleteResource.prototype.getStateKeyHash_asU8 = function() {
  return /** @type {!Uint8Array} */ (jspb.Message.bytesAsU8(
      this.getStateKeyHash()));
};


/**
 * @param {!(string|Uint8Array)} value
 * @return {!proto.aptos.transaction.testing1.v1.DeleteResource} returns this
 */
proto.aptos.transaction.testing1.v1.DeleteResource.prototype.setStateKeyHash = function(value) {
  return jspb.Message.setProto3BytesField(this, 2, value);
};


/**
 * optional MoveStructTag type = 3;
 * @return {?proto.aptos.transaction.testing1.v1.MoveStructTag}
 */
proto.aptos.transaction.testing1.v1.DeleteResource.prototype.getType = function() {
  return /** @type{?proto.aptos.transaction.testing1.v1.MoveStructTag} */ (
    jspb.Message.getWrapperField(this, proto.aptos.transaction.testing1.v1.MoveStructTag, 3));
};


/**
 * @param {?proto.aptos.transaction.testing1.v1.MoveStructTag|undefined} value
 * @return {!proto.aptos.transaction.testing1.v1.DeleteResource} returns this
*/
proto.aptos.transaction.testing1.v1.DeleteResource.prototype.setType = function(value) {
  return jspb.Message.setWrapperField(this, 3, value);
};


/**
 * Clears the message field making it undefined.
 * @return {!proto.aptos.transaction.testing1.v1.DeleteResource} returns this
 */
proto.aptos.transaction.testing1.v1.DeleteResource.prototype.clearType = function() {
  return this.setType(undefined);
};


/**
 * Returns whether this field is set.
 * @return {boolean}
 */
proto.aptos.transaction.testing1.v1.DeleteResource.prototype.hasType = function() {
  return jspb.Message.getField(this, 3) != null;
};


/**
 * optional string type_str = 4;
 * @return {string}
 */
proto.aptos.transaction.testing1.v1.DeleteResource.prototype.getTypeStr = function() {
  return /** @type {string} */ (jspb.Message.getFieldWithDefault(this, 4, ""));
};


/**
 * @param {string} value
 * @return {!proto.aptos.transaction.testing1.v1.DeleteResource} returns this
 */
proto.aptos.transaction.testing1.v1.DeleteResource.prototype.setTypeStr = function(value) {
  return jspb.Message.setProto3StringField(this, 4, value);
};





if (jspb.Message.GENERATE_TO_OBJECT) {
/**
 * Creates an object representation of this proto.
 * Field names that are reserved in JavaScript and will be renamed to pb_name.
 * Optional fields that are not set will be set to undefined.
 * To access a reserved field use, foo.pb_<name>, eg, foo.pb_default.
 * For the list of reserved names please see:
 *     net/proto2/compiler/js/internal/generator.cc#kKeyword.
 * @param {boolean=} opt_includeInstance Deprecated. whether to include the
 *     JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @return {!Object}
 */
proto.aptos.transaction.testing1.v1.DeleteTableItem.prototype.toObject = function(opt_includeInstance) {
  return proto.aptos.transaction.testing1.v1.DeleteTableItem.toObject(opt_includeInstance, this);
};


/**
 * Static version of the {@see toObject} method.
 * @param {boolean|undefined} includeInstance Deprecated. Whether to include
 *     the JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @param {!proto.aptos.transaction.testing1.v1.DeleteTableItem} msg The msg instance to transform.
 * @return {!Object}
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.aptos.transaction.testing1.v1.DeleteTableItem.toObject = function(includeInstance, msg) {
  var f, obj = {
    stateKeyHash: msg.getStateKeyHash_asB64(),
    handle: jspb.Message.getFieldWithDefault(msg, 2, ""),
    key: jspb.Message.getFieldWithDefault(msg, 3, ""),
    data: (f = msg.getData()) && proto.aptos.transaction.testing1.v1.DeleteTableData.toObject(includeInstance, f)
  };

  if (includeInstance) {
    obj.$jspbMessageInstance = msg;
  }
  return obj;
};
}


/**
 * Deserializes binary data (in protobuf wire format).
 * @param {jspb.ByteSource} bytes The bytes to deserialize.
 * @return {!proto.aptos.transaction.testing1.v1.DeleteTableItem}
 */
proto.aptos.transaction.testing1.v1.DeleteTableItem.deserializeBinary = function(bytes) {
  var reader = new jspb.BinaryReader(bytes);
  var msg = new proto.aptos.transaction.testing1.v1.DeleteTableItem;
  return proto.aptos.transaction.testing1.v1.DeleteTableItem.deserializeBinaryFromReader(msg, reader);
};


/**
 * Deserializes binary data (in protobuf wire format) from the
 * given reader into the given message object.
 * @param {!proto.aptos.transaction.testing1.v1.DeleteTableItem} msg The message object to deserialize into.
 * @param {!jspb.BinaryReader} reader The BinaryReader to use.
 * @return {!proto.aptos.transaction.testing1.v1.DeleteTableItem}
 */
proto.aptos.transaction.testing1.v1.DeleteTableItem.deserializeBinaryFromReader = function(msg, reader) {
  while (reader.nextField()) {
    if (reader.isEndGroup()) {
      break;
    }
    var field = reader.getFieldNumber();
    switch (field) {
    case 1:
      var value = /** @type {!Uint8Array} */ (reader.readBytes());
      msg.setStateKeyHash(value);
      break;
    case 2:
      var value = /** @type {string} */ (reader.readString());
      msg.setHandle(value);
      break;
    case 3:
      var value = /** @type {string} */ (reader.readString());
      msg.setKey(value);
      break;
    case 4:
      var value = new proto.aptos.transaction.testing1.v1.DeleteTableData;
      reader.readMessage(value,proto.aptos.transaction.testing1.v1.DeleteTableData.deserializeBinaryFromReader);
      msg.setData(value);
      break;
    default:
      reader.skipField();
      break;
    }
  }
  return msg;
};


/**
 * Serializes the message to binary data (in protobuf wire format).
 * @return {!Uint8Array}
 */
proto.aptos.transaction.testing1.v1.DeleteTableItem.prototype.serializeBinary = function() {
  var writer = new jspb.BinaryWriter();
  proto.aptos.transaction.testing1.v1.DeleteTableItem.serializeBinaryToWriter(this, writer);
  return writer.getResultBuffer();
};


/**
 * Serializes the given message to binary data (in protobuf wire
 * format), writing to the given BinaryWriter.
 * @param {!proto.aptos.transaction.testing1.v1.DeleteTableItem} message
 * @param {!jspb.BinaryWriter} writer
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.aptos.transaction.testing1.v1.DeleteTableItem.serializeBinaryToWriter = function(message, writer) {
  var f = undefined;
  f = message.getStateKeyHash_asU8();
  if (f.length > 0) {
    writer.writeBytes(
      1,
      f
    );
  }
  f = message.getHandle();
  if (f.length > 0) {
    writer.writeString(
      2,
      f
    );
  }
  f = message.getKey();
  if (f.length > 0) {
    writer.writeString(
      3,
      f
    );
  }
  f = message.getData();
  if (f != null) {
    writer.writeMessage(
      4,
      f,
      proto.aptos.transaction.testing1.v1.DeleteTableData.serializeBinaryToWriter
    );
  }
};


/**
 * optional bytes state_key_hash = 1;
 * @return {!(string|Uint8Array)}
 */
proto.aptos.transaction.testing1.v1.DeleteTableItem.prototype.getStateKeyHash = function() {
  return /** @type {!(string|Uint8Array)} */ (jspb.Message.getFieldWithDefault(this, 1, ""));
};


/**
 * optional bytes state_key_hash = 1;
 * This is a type-conversion wrapper around `getStateKeyHash()`
 * @return {string}
 */
proto.aptos.transaction.testing1.v1.DeleteTableItem.prototype.getStateKeyHash_asB64 = function() {
  return /** @type {string} */ (jspb.Message.bytesAsB64(
      this.getStateKeyHash()));
};


/**
 * optional bytes state_key_hash = 1;
 * Note that Uint8Array is not supported on all browsers.
 * @see http://caniuse.com/Uint8Array
 * This is a type-conversion wrapper around `getStateKeyHash()`
 * @return {!Uint8Array}
 */
proto.aptos.transaction.testing1.v1.DeleteTableItem.prototype.getStateKeyHash_asU8 = function() {
  return /** @type {!Uint8Array} */ (jspb.Message.bytesAsU8(
      this.getStateKeyHash()));
};


/**
 * @param {!(string|Uint8Array)} value
 * @return {!proto.aptos.transaction.testing1.v1.DeleteTableItem} returns this
 */
proto.aptos.transaction.testing1.v1.DeleteTableItem.prototype.setStateKeyHash = function(value) {
  return jspb.Message.setProto3BytesField(this, 1, value);
};


/**
 * optional string handle = 2;
 * @return {string}
 */
proto.aptos.transaction.testing1.v1.DeleteTableItem.prototype.getHandle = function() {
  return /** @type {string} */ (jspb.Message.getFieldWithDefault(this, 2, ""));
};


/**
 * @param {string} value
 * @return {!proto.aptos.transaction.testing1.v1.DeleteTableItem} returns this
 */
proto.aptos.transaction.testing1.v1.DeleteTableItem.prototype.setHandle = function(value) {
  return jspb.Message.setProto3StringField(this, 2, value);
};


/**
 * optional string key = 3;
 * @return {string}
 */
proto.aptos.transaction.testing1.v1.DeleteTableItem.prototype.getKey = function() {
  return /** @type {string} */ (jspb.Message.getFieldWithDefault(this, 3, ""));
};


/**
 * @param {string} value
 * @return {!proto.aptos.transaction.testing1.v1.DeleteTableItem} returns this
 */
proto.aptos.transaction.testing1.v1.DeleteTableItem.prototype.setKey = function(value) {
  return jspb.Message.setProto3StringField(this, 3, value);
};


/**
 * optional DeleteTableData data = 4;
 * @return {?proto.aptos.transaction.testing1.v1.DeleteTableData}
 */
proto.aptos.transaction.testing1.v1.DeleteTableItem.prototype.getData = function() {
  return /** @type{?proto.aptos.transaction.testing1.v1.DeleteTableData} */ (
    jspb.Message.getWrapperField(this, proto.aptos.transaction.testing1.v1.DeleteTableData, 4));
};


/**
 * @param {?proto.aptos.transaction.testing1.v1.DeleteTableData|undefined} value
 * @return {!proto.aptos.transaction.testing1.v1.DeleteTableItem} returns this
*/
proto.aptos.transaction.testing1.v1.DeleteTableItem.prototype.setData = function(value) {
  return jspb.Message.setWrapperField(this, 4, value);
};


/**
 * Clears the message field making it undefined.
 * @return {!proto.aptos.transaction.testing1.v1.DeleteTableItem} returns this
 */
proto.aptos.transaction.testing1.v1.DeleteTableItem.prototype.clearData = function() {
  return this.setData(undefined);
};


/**
 * Returns whether this field is set.
 * @return {boolean}
 */
proto.aptos.transaction.testing1.v1.DeleteTableItem.prototype.hasData = function() {
  return jspb.Message.getField(this, 4) != null;
};





if (jspb.Message.GENERATE_TO_OBJECT) {
/**
 * Creates an object representation of this proto.
 * Field names that are reserved in JavaScript and will be renamed to pb_name.
 * Optional fields that are not set will be set to undefined.
 * To access a reserved field use, foo.pb_<name>, eg, foo.pb_default.
 * For the list of reserved names please see:
 *     net/proto2/compiler/js/internal/generator.cc#kKeyword.
 * @param {boolean=} opt_includeInstance Deprecated. whether to include the
 *     JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @return {!Object}
 */
proto.aptos.transaction.testing1.v1.DeleteTableData.prototype.toObject = function(opt_includeInstance) {
  return proto.aptos.transaction.testing1.v1.DeleteTableData.toObject(opt_includeInstance, this);
};


/**
 * Static version of the {@see toObject} method.
 * @param {boolean|undefined} includeInstance Deprecated. Whether to include
 *     the JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @param {!proto.aptos.transaction.testing1.v1.DeleteTableData} msg The msg instance to transform.
 * @return {!Object}
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.aptos.transaction.testing1.v1.DeleteTableData.toObject = function(includeInstance, msg) {
  var f, obj = {
    key: jspb.Message.getFieldWithDefault(msg, 1, ""),
    keyType: jspb.Message.getFieldWithDefault(msg, 2, "")
  };

  if (includeInstance) {
    obj.$jspbMessageInstance = msg;
  }
  return obj;
};
}


/**
 * Deserializes binary data (in protobuf wire format).
 * @param {jspb.ByteSource} bytes The bytes to deserialize.
 * @return {!proto.aptos.transaction.testing1.v1.DeleteTableData}
 */
proto.aptos.transaction.testing1.v1.DeleteTableData.deserializeBinary = function(bytes) {
  var reader = new jspb.BinaryReader(bytes);
  var msg = new proto.aptos.transaction.testing1.v1.DeleteTableData;
  return proto.aptos.transaction.testing1.v1.DeleteTableData.deserializeBinaryFromReader(msg, reader);
};


/**
 * Deserializes binary data (in protobuf wire format) from the
 * given reader into the given message object.
 * @param {!proto.aptos.transaction.testing1.v1.DeleteTableData} msg The message object to deserialize into.
 * @param {!jspb.BinaryReader} reader The BinaryReader to use.
 * @return {!proto.aptos.transaction.testing1.v1.DeleteTableData}
 */
proto.aptos.transaction.testing1.v1.DeleteTableData.deserializeBinaryFromReader = function(msg, reader) {
  while (reader.nextField()) {
    if (reader.isEndGroup()) {
      break;
    }
    var field = reader.getFieldNumber();
    switch (field) {
    case 1:
      var value = /** @type {string} */ (reader.readString());
      msg.setKey(value);
      break;
    case 2:
      var value = /** @type {string} */ (reader.readString());
      msg.setKeyType(value);
      break;
    default:
      reader.skipField();
      break;
    }
  }
  return msg;
};


/**
 * Serializes the message to binary data (in protobuf wire format).
 * @return {!Uint8Array}
 */
proto.aptos.transaction.testing1.v1.DeleteTableData.prototype.serializeBinary = function() {
  var writer = new jspb.BinaryWriter();
  proto.aptos.transaction.testing1.v1.DeleteTableData.serializeBinaryToWriter(this, writer);
  return writer.getResultBuffer();
};


/**
 * Serializes the given message to binary data (in protobuf wire
 * format), writing to the given BinaryWriter.
 * @param {!proto.aptos.transaction.testing1.v1.DeleteTableData} message
 * @param {!jspb.BinaryWriter} writer
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.aptos.transaction.testing1.v1.DeleteTableData.serializeBinaryToWriter = function(message, writer) {
  var f = undefined;
  f = message.getKey();
  if (f.length > 0) {
    writer.writeString(
      1,
      f
    );
  }
  f = message.getKeyType();
  if (f.length > 0) {
    writer.writeString(
      2,
      f
    );
  }
};


/**
 * optional string key = 1;
 * @return {string}
 */
proto.aptos.transaction.testing1.v1.DeleteTableData.prototype.getKey = function() {
  return /** @type {string} */ (jspb.Message.getFieldWithDefault(this, 1, ""));
};


/**
 * @param {string} value
 * @return {!proto.aptos.transaction.testing1.v1.DeleteTableData} returns this
 */
proto.aptos.transaction.testing1.v1.DeleteTableData.prototype.setKey = function(value) {
  return jspb.Message.setProto3StringField(this, 1, value);
};


/**
 * optional string key_type = 2;
 * @return {string}
 */
proto.aptos.transaction.testing1.v1.DeleteTableData.prototype.getKeyType = function() {
  return /** @type {string} */ (jspb.Message.getFieldWithDefault(this, 2, ""));
};


/**
 * @param {string} value
 * @return {!proto.aptos.transaction.testing1.v1.DeleteTableData} returns this
 */
proto.aptos.transaction.testing1.v1.DeleteTableData.prototype.setKeyType = function(value) {
  return jspb.Message.setProto3StringField(this, 2, value);
};





if (jspb.Message.GENERATE_TO_OBJECT) {
/**
 * Creates an object representation of this proto.
 * Field names that are reserved in JavaScript and will be renamed to pb_name.
 * Optional fields that are not set will be set to undefined.
 * To access a reserved field use, foo.pb_<name>, eg, foo.pb_default.
 * For the list of reserved names please see:
 *     net/proto2/compiler/js/internal/generator.cc#kKeyword.
 * @param {boolean=} opt_includeInstance Deprecated. whether to include the
 *     JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @return {!Object}
 */
proto.aptos.transaction.testing1.v1.WriteModule.prototype.toObject = function(opt_includeInstance) {
  return proto.aptos.transaction.testing1.v1.WriteModule.toObject(opt_includeInstance, this);
};


/**
 * Static version of the {@see toObject} method.
 * @param {boolean|undefined} includeInstance Deprecated. Whether to include
 *     the JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @param {!proto.aptos.transaction.testing1.v1.WriteModule} msg The msg instance to transform.
 * @return {!Object}
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.aptos.transaction.testing1.v1.WriteModule.toObject = function(includeInstance, msg) {
  var f, obj = {
    address: jspb.Message.getFieldWithDefault(msg, 1, ""),
    stateKeyHash: msg.getStateKeyHash_asB64(),
    data: (f = msg.getData()) && proto.aptos.transaction.testing1.v1.MoveModuleBytecode.toObject(includeInstance, f)
  };

  if (includeInstance) {
    obj.$jspbMessageInstance = msg;
  }
  return obj;
};
}


/**
 * Deserializes binary data (in protobuf wire format).
 * @param {jspb.ByteSource} bytes The bytes to deserialize.
 * @return {!proto.aptos.transaction.testing1.v1.WriteModule}
 */
proto.aptos.transaction.testing1.v1.WriteModule.deserializeBinary = function(bytes) {
  var reader = new jspb.BinaryReader(bytes);
  var msg = new proto.aptos.transaction.testing1.v1.WriteModule;
  return proto.aptos.transaction.testing1.v1.WriteModule.deserializeBinaryFromReader(msg, reader);
};


/**
 * Deserializes binary data (in protobuf wire format) from the
 * given reader into the given message object.
 * @param {!proto.aptos.transaction.testing1.v1.WriteModule} msg The message object to deserialize into.
 * @param {!jspb.BinaryReader} reader The BinaryReader to use.
 * @return {!proto.aptos.transaction.testing1.v1.WriteModule}
 */
proto.aptos.transaction.testing1.v1.WriteModule.deserializeBinaryFromReader = function(msg, reader) {
  while (reader.nextField()) {
    if (reader.isEndGroup()) {
      break;
    }
    var field = reader.getFieldNumber();
    switch (field) {
    case 1:
      var value = /** @type {string} */ (reader.readString());
      msg.setAddress(value);
      break;
    case 2:
      var value = /** @type {!Uint8Array} */ (reader.readBytes());
      msg.setStateKeyHash(value);
      break;
    case 3:
      var value = new proto.aptos.transaction.testing1.v1.MoveModuleBytecode;
      reader.readMessage(value,proto.aptos.transaction.testing1.v1.MoveModuleBytecode.deserializeBinaryFromReader);
      msg.setData(value);
      break;
    default:
      reader.skipField();
      break;
    }
  }
  return msg;
};


/**
 * Serializes the message to binary data (in protobuf wire format).
 * @return {!Uint8Array}
 */
proto.aptos.transaction.testing1.v1.WriteModule.prototype.serializeBinary = function() {
  var writer = new jspb.BinaryWriter();
  proto.aptos.transaction.testing1.v1.WriteModule.serializeBinaryToWriter(this, writer);
  return writer.getResultBuffer();
};


/**
 * Serializes the given message to binary data (in protobuf wire
 * format), writing to the given BinaryWriter.
 * @param {!proto.aptos.transaction.testing1.v1.WriteModule} message
 * @param {!jspb.BinaryWriter} writer
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.aptos.transaction.testing1.v1.WriteModule.serializeBinaryToWriter = function(message, writer) {
  var f = undefined;
  f = message.getAddress();
  if (f.length > 0) {
    writer.writeString(
      1,
      f
    );
  }
  f = message.getStateKeyHash_asU8();
  if (f.length > 0) {
    writer.writeBytes(
      2,
      f
    );
  }
  f = message.getData();
  if (f != null) {
    writer.writeMessage(
      3,
      f,
      proto.aptos.transaction.testing1.v1.MoveModuleBytecode.serializeBinaryToWriter
    );
  }
};


/**
 * optional string address = 1;
 * @return {string}
 */
proto.aptos.transaction.testing1.v1.WriteModule.prototype.getAddress = function() {
  return /** @type {string} */ (jspb.Message.getFieldWithDefault(this, 1, ""));
};


/**
 * @param {string} value
 * @return {!proto.aptos.transaction.testing1.v1.WriteModule} returns this
 */
proto.aptos.transaction.testing1.v1.WriteModule.prototype.setAddress = function(value) {
  return jspb.Message.setProto3StringField(this, 1, value);
};


/**
 * optional bytes state_key_hash = 2;
 * @return {!(string|Uint8Array)}
 */
proto.aptos.transaction.testing1.v1.WriteModule.prototype.getStateKeyHash = function() {
  return /** @type {!(string|Uint8Array)} */ (jspb.Message.getFieldWithDefault(this, 2, ""));
};


/**
 * optional bytes state_key_hash = 2;
 * This is a type-conversion wrapper around `getStateKeyHash()`
 * @return {string}
 */
proto.aptos.transaction.testing1.v1.WriteModule.prototype.getStateKeyHash_asB64 = function() {
  return /** @type {string} */ (jspb.Message.bytesAsB64(
      this.getStateKeyHash()));
};


/**
 * optional bytes state_key_hash = 2;
 * Note that Uint8Array is not supported on all browsers.
 * @see http://caniuse.com/Uint8Array
 * This is a type-conversion wrapper around `getStateKeyHash()`
 * @return {!Uint8Array}
 */
proto.aptos.transaction.testing1.v1.WriteModule.prototype.getStateKeyHash_asU8 = function() {
  return /** @type {!Uint8Array} */ (jspb.Message.bytesAsU8(
      this.getStateKeyHash()));
};


/**
 * @param {!(string|Uint8Array)} value
 * @return {!proto.aptos.transaction.testing1.v1.WriteModule} returns this
 */
proto.aptos.transaction.testing1.v1.WriteModule.prototype.setStateKeyHash = function(value) {
  return jspb.Message.setProto3BytesField(this, 2, value);
};


/**
 * optional MoveModuleBytecode data = 3;
 * @return {?proto.aptos.transaction.testing1.v1.MoveModuleBytecode}
 */
proto.aptos.transaction.testing1.v1.WriteModule.prototype.getData = function() {
  return /** @type{?proto.aptos.transaction.testing1.v1.MoveModuleBytecode} */ (
    jspb.Message.getWrapperField(this, proto.aptos.transaction.testing1.v1.MoveModuleBytecode, 3));
};


/**
 * @param {?proto.aptos.transaction.testing1.v1.MoveModuleBytecode|undefined} value
 * @return {!proto.aptos.transaction.testing1.v1.WriteModule} returns this
*/
proto.aptos.transaction.testing1.v1.WriteModule.prototype.setData = function(value) {
  return jspb.Message.setWrapperField(this, 3, value);
};


/**
 * Clears the message field making it undefined.
 * @return {!proto.aptos.transaction.testing1.v1.WriteModule} returns this
 */
proto.aptos.transaction.testing1.v1.WriteModule.prototype.clearData = function() {
  return this.setData(undefined);
};


/**
 * Returns whether this field is set.
 * @return {boolean}
 */
proto.aptos.transaction.testing1.v1.WriteModule.prototype.hasData = function() {
  return jspb.Message.getField(this, 3) != null;
};





if (jspb.Message.GENERATE_TO_OBJECT) {
/**
 * Creates an object representation of this proto.
 * Field names that are reserved in JavaScript and will be renamed to pb_name.
 * Optional fields that are not set will be set to undefined.
 * To access a reserved field use, foo.pb_<name>, eg, foo.pb_default.
 * For the list of reserved names please see:
 *     net/proto2/compiler/js/internal/generator.cc#kKeyword.
 * @param {boolean=} opt_includeInstance Deprecated. whether to include the
 *     JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @return {!Object}
 */
proto.aptos.transaction.testing1.v1.WriteResource.prototype.toObject = function(opt_includeInstance) {
  return proto.aptos.transaction.testing1.v1.WriteResource.toObject(opt_includeInstance, this);
};


/**
 * Static version of the {@see toObject} method.
 * @param {boolean|undefined} includeInstance Deprecated. Whether to include
 *     the JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @param {!proto.aptos.transaction.testing1.v1.WriteResource} msg The msg instance to transform.
 * @return {!Object}
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.aptos.transaction.testing1.v1.WriteResource.toObject = function(includeInstance, msg) {
  var f, obj = {
    address: jspb.Message.getFieldWithDefault(msg, 1, ""),
    stateKeyHash: msg.getStateKeyHash_asB64(),
    type: (f = msg.getType()) && proto.aptos.transaction.testing1.v1.MoveStructTag.toObject(includeInstance, f),
    typeStr: jspb.Message.getFieldWithDefault(msg, 4, ""),
    data: jspb.Message.getFieldWithDefault(msg, 5, "")
  };

  if (includeInstance) {
    obj.$jspbMessageInstance = msg;
  }
  return obj;
};
}


/**
 * Deserializes binary data (in protobuf wire format).
 * @param {jspb.ByteSource} bytes The bytes to deserialize.
 * @return {!proto.aptos.transaction.testing1.v1.WriteResource}
 */
proto.aptos.transaction.testing1.v1.WriteResource.deserializeBinary = function(bytes) {
  var reader = new jspb.BinaryReader(bytes);
  var msg = new proto.aptos.transaction.testing1.v1.WriteResource;
  return proto.aptos.transaction.testing1.v1.WriteResource.deserializeBinaryFromReader(msg, reader);
};


/**
 * Deserializes binary data (in protobuf wire format) from the
 * given reader into the given message object.
 * @param {!proto.aptos.transaction.testing1.v1.WriteResource} msg The message object to deserialize into.
 * @param {!jspb.BinaryReader} reader The BinaryReader to use.
 * @return {!proto.aptos.transaction.testing1.v1.WriteResource}
 */
proto.aptos.transaction.testing1.v1.WriteResource.deserializeBinaryFromReader = function(msg, reader) {
  while (reader.nextField()) {
    if (reader.isEndGroup()) {
      break;
    }
    var field = reader.getFieldNumber();
    switch (field) {
    case 1:
      var value = /** @type {string} */ (reader.readString());
      msg.setAddress(value);
      break;
    case 2:
      var value = /** @type {!Uint8Array} */ (reader.readBytes());
      msg.setStateKeyHash(value);
      break;
    case 3:
      var value = new proto.aptos.transaction.testing1.v1.MoveStructTag;
      reader.readMessage(value,proto.aptos.transaction.testing1.v1.MoveStructTag.deserializeBinaryFromReader);
      msg.setType(value);
      break;
    case 4:
      var value = /** @type {string} */ (reader.readString());
      msg.setTypeStr(value);
      break;
    case 5:
      var value = /** @type {string} */ (reader.readString());
      msg.setData(value);
      break;
    default:
      reader.skipField();
      break;
    }
  }
  return msg;
};


/**
 * Serializes the message to binary data (in protobuf wire format).
 * @return {!Uint8Array}
 */
proto.aptos.transaction.testing1.v1.WriteResource.prototype.serializeBinary = function() {
  var writer = new jspb.BinaryWriter();
  proto.aptos.transaction.testing1.v1.WriteResource.serializeBinaryToWriter(this, writer);
  return writer.getResultBuffer();
};


/**
 * Serializes the given message to binary data (in protobuf wire
 * format), writing to the given BinaryWriter.
 * @param {!proto.aptos.transaction.testing1.v1.WriteResource} message
 * @param {!jspb.BinaryWriter} writer
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.aptos.transaction.testing1.v1.WriteResource.serializeBinaryToWriter = function(message, writer) {
  var f = undefined;
  f = message.getAddress();
  if (f.length > 0) {
    writer.writeString(
      1,
      f
    );
  }
  f = message.getStateKeyHash_asU8();
  if (f.length > 0) {
    writer.writeBytes(
      2,
      f
    );
  }
  f = message.getType();
  if (f != null) {
    writer.writeMessage(
      3,
      f,
      proto.aptos.transaction.testing1.v1.MoveStructTag.serializeBinaryToWriter
    );
  }
  f = message.getTypeStr();
  if (f.length > 0) {
    writer.writeString(
      4,
      f
    );
  }
  f = message.getData();
  if (f.length > 0) {
    writer.writeString(
      5,
      f
    );
  }
};


/**
 * optional string address = 1;
 * @return {string}
 */
proto.aptos.transaction.testing1.v1.WriteResource.prototype.getAddress = function() {
  return /** @type {string} */ (jspb.Message.getFieldWithDefault(this, 1, ""));
};


/**
 * @param {string} value
 * @return {!proto.aptos.transaction.testing1.v1.WriteResource} returns this
 */
proto.aptos.transaction.testing1.v1.WriteResource.prototype.setAddress = function(value) {
  return jspb.Message.setProto3StringField(this, 1, value);
};


/**
 * optional bytes state_key_hash = 2;
 * @return {!(string|Uint8Array)}
 */
proto.aptos.transaction.testing1.v1.WriteResource.prototype.getStateKeyHash = function() {
  return /** @type {!(string|Uint8Array)} */ (jspb.Message.getFieldWithDefault(this, 2, ""));
};


/**
 * optional bytes state_key_hash = 2;
 * This is a type-conversion wrapper around `getStateKeyHash()`
 * @return {string}
 */
proto.aptos.transaction.testing1.v1.WriteResource.prototype.getStateKeyHash_asB64 = function() {
  return /** @type {string} */ (jspb.Message.bytesAsB64(
      this.getStateKeyHash()));
};


/**
 * optional bytes state_key_hash = 2;
 * Note that Uint8Array is not supported on all browsers.
 * @see http://caniuse.com/Uint8Array
 * This is a type-conversion wrapper around `getStateKeyHash()`
 * @return {!Uint8Array}
 */
proto.aptos.transaction.testing1.v1.WriteResource.prototype.getStateKeyHash_asU8 = function() {
  return /** @type {!Uint8Array} */ (jspb.Message.bytesAsU8(
      this.getStateKeyHash()));
};


/**
 * @param {!(string|Uint8Array)} value
 * @return {!proto.aptos.transaction.testing1.v1.WriteResource} returns this
 */
proto.aptos.transaction.testing1.v1.WriteResource.prototype.setStateKeyHash = function(value) {
  return jspb.Message.setProto3BytesField(this, 2, value);
};


/**
 * optional MoveStructTag type = 3;
 * @return {?proto.aptos.transaction.testing1.v1.MoveStructTag}
 */
proto.aptos.transaction.testing1.v1.WriteResource.prototype.getType = function() {
  return /** @type{?proto.aptos.transaction.testing1.v1.MoveStructTag} */ (
    jspb.Message.getWrapperField(this, proto.aptos.transaction.testing1.v1.MoveStructTag, 3));
};


/**
 * @param {?proto.aptos.transaction.testing1.v1.MoveStructTag|undefined} value
 * @return {!proto.aptos.transaction.testing1.v1.WriteResource} returns this
*/
proto.aptos.transaction.testing1.v1.WriteResource.prototype.setType = function(value) {
  return jspb.Message.setWrapperField(this, 3, value);
};


/**
 * Clears the message field making it undefined.
 * @return {!proto.aptos.transaction.testing1.v1.WriteResource} returns this
 */
proto.aptos.transaction.testing1.v1.WriteResource.prototype.clearType = function() {
  return this.setType(undefined);
};


/**
 * Returns whether this field is set.
 * @return {boolean}
 */
proto.aptos.transaction.testing1.v1.WriteResource.prototype.hasType = function() {
  return jspb.Message.getField(this, 3) != null;
};


/**
 * optional string type_str = 4;
 * @return {string}
 */
proto.aptos.transaction.testing1.v1.WriteResource.prototype.getTypeStr = function() {
  return /** @type {string} */ (jspb.Message.getFieldWithDefault(this, 4, ""));
};


/**
 * @param {string} value
 * @return {!proto.aptos.transaction.testing1.v1.WriteResource} returns this
 */
proto.aptos.transaction.testing1.v1.WriteResource.prototype.setTypeStr = function(value) {
  return jspb.Message.setProto3StringField(this, 4, value);
};


/**
 * optional string data = 5;
 * @return {string}
 */
proto.aptos.transaction.testing1.v1.WriteResource.prototype.getData = function() {
  return /** @type {string} */ (jspb.Message.getFieldWithDefault(this, 5, ""));
};


/**
 * @param {string} value
 * @return {!proto.aptos.transaction.testing1.v1.WriteResource} returns this
 */
proto.aptos.transaction.testing1.v1.WriteResource.prototype.setData = function(value) {
  return jspb.Message.setProto3StringField(this, 5, value);
};





if (jspb.Message.GENERATE_TO_OBJECT) {
/**
 * Creates an object representation of this proto.
 * Field names that are reserved in JavaScript and will be renamed to pb_name.
 * Optional fields that are not set will be set to undefined.
 * To access a reserved field use, foo.pb_<name>, eg, foo.pb_default.
 * For the list of reserved names please see:
 *     net/proto2/compiler/js/internal/generator.cc#kKeyword.
 * @param {boolean=} opt_includeInstance Deprecated. whether to include the
 *     JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @return {!Object}
 */
proto.aptos.transaction.testing1.v1.WriteTableData.prototype.toObject = function(opt_includeInstance) {
  return proto.aptos.transaction.testing1.v1.WriteTableData.toObject(opt_includeInstance, this);
};


/**
 * Static version of the {@see toObject} method.
 * @param {boolean|undefined} includeInstance Deprecated. Whether to include
 *     the JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @param {!proto.aptos.transaction.testing1.v1.WriteTableData} msg The msg instance to transform.
 * @return {!Object}
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.aptos.transaction.testing1.v1.WriteTableData.toObject = function(includeInstance, msg) {
  var f, obj = {
    key: jspb.Message.getFieldWithDefault(msg, 1, ""),
    keyType: jspb.Message.getFieldWithDefault(msg, 2, ""),
    value: jspb.Message.getFieldWithDefault(msg, 3, ""),
    valueType: jspb.Message.getFieldWithDefault(msg, 4, "")
  };

  if (includeInstance) {
    obj.$jspbMessageInstance = msg;
  }
  return obj;
};
}


/**
 * Deserializes binary data (in protobuf wire format).
 * @param {jspb.ByteSource} bytes The bytes to deserialize.
 * @return {!proto.aptos.transaction.testing1.v1.WriteTableData}
 */
proto.aptos.transaction.testing1.v1.WriteTableData.deserializeBinary = function(bytes) {
  var reader = new jspb.BinaryReader(bytes);
  var msg = new proto.aptos.transaction.testing1.v1.WriteTableData;
  return proto.aptos.transaction.testing1.v1.WriteTableData.deserializeBinaryFromReader(msg, reader);
};


/**
 * Deserializes binary data (in protobuf wire format) from the
 * given reader into the given message object.
 * @param {!proto.aptos.transaction.testing1.v1.WriteTableData} msg The message object to deserialize into.
 * @param {!jspb.BinaryReader} reader The BinaryReader to use.
 * @return {!proto.aptos.transaction.testing1.v1.WriteTableData}
 */
proto.aptos.transaction.testing1.v1.WriteTableData.deserializeBinaryFromReader = function(msg, reader) {
  while (reader.nextField()) {
    if (reader.isEndGroup()) {
      break;
    }
    var field = reader.getFieldNumber();
    switch (field) {
    case 1:
      var value = /** @type {string} */ (reader.readString());
      msg.setKey(value);
      break;
    case 2:
      var value = /** @type {string} */ (reader.readString());
      msg.setKeyType(value);
      break;
    case 3:
      var value = /** @type {string} */ (reader.readString());
      msg.setValue(value);
      break;
    case 4:
      var value = /** @type {string} */ (reader.readString());
      msg.setValueType(value);
      break;
    default:
      reader.skipField();
      break;
    }
  }
  return msg;
};


/**
 * Serializes the message to binary data (in protobuf wire format).
 * @return {!Uint8Array}
 */
proto.aptos.transaction.testing1.v1.WriteTableData.prototype.serializeBinary = function() {
  var writer = new jspb.BinaryWriter();
  proto.aptos.transaction.testing1.v1.WriteTableData.serializeBinaryToWriter(this, writer);
  return writer.getResultBuffer();
};


/**
 * Serializes the given message to binary data (in protobuf wire
 * format), writing to the given BinaryWriter.
 * @param {!proto.aptos.transaction.testing1.v1.WriteTableData} message
 * @param {!jspb.BinaryWriter} writer
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.aptos.transaction.testing1.v1.WriteTableData.serializeBinaryToWriter = function(message, writer) {
  var f = undefined;
  f = message.getKey();
  if (f.length > 0) {
    writer.writeString(
      1,
      f
    );
  }
  f = message.getKeyType();
  if (f.length > 0) {
    writer.writeString(
      2,
      f
    );
  }
  f = message.getValue();
  if (f.length > 0) {
    writer.writeString(
      3,
      f
    );
  }
  f = message.getValueType();
  if (f.length > 0) {
    writer.writeString(
      4,
      f
    );
  }
};


/**
 * optional string key = 1;
 * @return {string}
 */
proto.aptos.transaction.testing1.v1.WriteTableData.prototype.getKey = function() {
  return /** @type {string} */ (jspb.Message.getFieldWithDefault(this, 1, ""));
};


/**
 * @param {string} value
 * @return {!proto.aptos.transaction.testing1.v1.WriteTableData} returns this
 */
proto.aptos.transaction.testing1.v1.WriteTableData.prototype.setKey = function(value) {
  return jspb.Message.setProto3StringField(this, 1, value);
};


/**
 * optional string key_type = 2;
 * @return {string}
 */
proto.aptos.transaction.testing1.v1.WriteTableData.prototype.getKeyType = function() {
  return /** @type {string} */ (jspb.Message.getFieldWithDefault(this, 2, ""));
};


/**
 * @param {string} value
 * @return {!proto.aptos.transaction.testing1.v1.WriteTableData} returns this
 */
proto.aptos.transaction.testing1.v1.WriteTableData.prototype.setKeyType = function(value) {
  return jspb.Message.setProto3StringField(this, 2, value);
};


/**
 * optional string value = 3;
 * @return {string}
 */
proto.aptos.transaction.testing1.v1.WriteTableData.prototype.getValue = function() {
  return /** @type {string} */ (jspb.Message.getFieldWithDefault(this, 3, ""));
};


/**
 * @param {string} value
 * @return {!proto.aptos.transaction.testing1.v1.WriteTableData} returns this
 */
proto.aptos.transaction.testing1.v1.WriteTableData.prototype.setValue = function(value) {
  return jspb.Message.setProto3StringField(this, 3, value);
};


/**
 * optional string value_type = 4;
 * @return {string}
 */
proto.aptos.transaction.testing1.v1.WriteTableData.prototype.getValueType = function() {
  return /** @type {string} */ (jspb.Message.getFieldWithDefault(this, 4, ""));
};


/**
 * @param {string} value
 * @return {!proto.aptos.transaction.testing1.v1.WriteTableData} returns this
 */
proto.aptos.transaction.testing1.v1.WriteTableData.prototype.setValueType = function(value) {
  return jspb.Message.setProto3StringField(this, 4, value);
};





if (jspb.Message.GENERATE_TO_OBJECT) {
/**
 * Creates an object representation of this proto.
 * Field names that are reserved in JavaScript and will be renamed to pb_name.
 * Optional fields that are not set will be set to undefined.
 * To access a reserved field use, foo.pb_<name>, eg, foo.pb_default.
 * For the list of reserved names please see:
 *     net/proto2/compiler/js/internal/generator.cc#kKeyword.
 * @param {boolean=} opt_includeInstance Deprecated. whether to include the
 *     JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @return {!Object}
 */
proto.aptos.transaction.testing1.v1.WriteTableItem.prototype.toObject = function(opt_includeInstance) {
  return proto.aptos.transaction.testing1.v1.WriteTableItem.toObject(opt_includeInstance, this);
};


/**
 * Static version of the {@see toObject} method.
 * @param {boolean|undefined} includeInstance Deprecated. Whether to include
 *     the JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @param {!proto.aptos.transaction.testing1.v1.WriteTableItem} msg The msg instance to transform.
 * @return {!Object}
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.aptos.transaction.testing1.v1.WriteTableItem.toObject = function(includeInstance, msg) {
  var f, obj = {
    stateKeyHash: msg.getStateKeyHash_asB64(),
    handle: jspb.Message.getFieldWithDefault(msg, 2, ""),
    key: jspb.Message.getFieldWithDefault(msg, 3, ""),
    data: (f = msg.getData()) && proto.aptos.transaction.testing1.v1.WriteTableData.toObject(includeInstance, f)
  };

  if (includeInstance) {
    obj.$jspbMessageInstance = msg;
  }
  return obj;
};
}


/**
 * Deserializes binary data (in protobuf wire format).
 * @param {jspb.ByteSource} bytes The bytes to deserialize.
 * @return {!proto.aptos.transaction.testing1.v1.WriteTableItem}
 */
proto.aptos.transaction.testing1.v1.WriteTableItem.deserializeBinary = function(bytes) {
  var reader = new jspb.BinaryReader(bytes);
  var msg = new proto.aptos.transaction.testing1.v1.WriteTableItem;
  return proto.aptos.transaction.testing1.v1.WriteTableItem.deserializeBinaryFromReader(msg, reader);
};


/**
 * Deserializes binary data (in protobuf wire format) from the
 * given reader into the given message object.
 * @param {!proto.aptos.transaction.testing1.v1.WriteTableItem} msg The message object to deserialize into.
 * @param {!jspb.BinaryReader} reader The BinaryReader to use.
 * @return {!proto.aptos.transaction.testing1.v1.WriteTableItem}
 */
proto.aptos.transaction.testing1.v1.WriteTableItem.deserializeBinaryFromReader = function(msg, reader) {
  while (reader.nextField()) {
    if (reader.isEndGroup()) {
      break;
    }
    var field = reader.getFieldNumber();
    switch (field) {
    case 1:
      var value = /** @type {!Uint8Array} */ (reader.readBytes());
      msg.setStateKeyHash(value);
      break;
    case 2:
      var value = /** @type {string} */ (reader.readString());
      msg.setHandle(value);
      break;
    case 3:
      var value = /** @type {string} */ (reader.readString());
      msg.setKey(value);
      break;
    case 4:
      var value = new proto.aptos.transaction.testing1.v1.WriteTableData;
      reader.readMessage(value,proto.aptos.transaction.testing1.v1.WriteTableData.deserializeBinaryFromReader);
      msg.setData(value);
      break;
    default:
      reader.skipField();
      break;
    }
  }
  return msg;
};


/**
 * Serializes the message to binary data (in protobuf wire format).
 * @return {!Uint8Array}
 */
proto.aptos.transaction.testing1.v1.WriteTableItem.prototype.serializeBinary = function() {
  var writer = new jspb.BinaryWriter();
  proto.aptos.transaction.testing1.v1.WriteTableItem.serializeBinaryToWriter(this, writer);
  return writer.getResultBuffer();
};


/**
 * Serializes the given message to binary data (in protobuf wire
 * format), writing to the given BinaryWriter.
 * @param {!proto.aptos.transaction.testing1.v1.WriteTableItem} message
 * @param {!jspb.BinaryWriter} writer
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.aptos.transaction.testing1.v1.WriteTableItem.serializeBinaryToWriter = function(message, writer) {
  var f = undefined;
  f = message.getStateKeyHash_asU8();
  if (f.length > 0) {
    writer.writeBytes(
      1,
      f
    );
  }
  f = message.getHandle();
  if (f.length > 0) {
    writer.writeString(
      2,
      f
    );
  }
  f = message.getKey();
  if (f.length > 0) {
    writer.writeString(
      3,
      f
    );
  }
  f = message.getData();
  if (f != null) {
    writer.writeMessage(
      4,
      f,
      proto.aptos.transaction.testing1.v1.WriteTableData.serializeBinaryToWriter
    );
  }
};


/**
 * optional bytes state_key_hash = 1;
 * @return {!(string|Uint8Array)}
 */
proto.aptos.transaction.testing1.v1.WriteTableItem.prototype.getStateKeyHash = function() {
  return /** @type {!(string|Uint8Array)} */ (jspb.Message.getFieldWithDefault(this, 1, ""));
};


/**
 * optional bytes state_key_hash = 1;
 * This is a type-conversion wrapper around `getStateKeyHash()`
 * @return {string}
 */
proto.aptos.transaction.testing1.v1.WriteTableItem.prototype.getStateKeyHash_asB64 = function() {
  return /** @type {string} */ (jspb.Message.bytesAsB64(
      this.getStateKeyHash()));
};


/**
 * optional bytes state_key_hash = 1;
 * Note that Uint8Array is not supported on all browsers.
 * @see http://caniuse.com/Uint8Array
 * This is a type-conversion wrapper around `getStateKeyHash()`
 * @return {!Uint8Array}
 */
proto.aptos.transaction.testing1.v1.WriteTableItem.prototype.getStateKeyHash_asU8 = function() {
  return /** @type {!Uint8Array} */ (jspb.Message.bytesAsU8(
      this.getStateKeyHash()));
};


/**
 * @param {!(string|Uint8Array)} value
 * @return {!proto.aptos.transaction.testing1.v1.WriteTableItem} returns this
 */
proto.aptos.transaction.testing1.v1.WriteTableItem.prototype.setStateKeyHash = function(value) {
  return jspb.Message.setProto3BytesField(this, 1, value);
};


/**
 * optional string handle = 2;
 * @return {string}
 */
proto.aptos.transaction.testing1.v1.WriteTableItem.prototype.getHandle = function() {
  return /** @type {string} */ (jspb.Message.getFieldWithDefault(this, 2, ""));
};


/**
 * @param {string} value
 * @return {!proto.aptos.transaction.testing1.v1.WriteTableItem} returns this
 */
proto.aptos.transaction.testing1.v1.WriteTableItem.prototype.setHandle = function(value) {
  return jspb.Message.setProto3StringField(this, 2, value);
};


/**
 * optional string key = 3;
 * @return {string}
 */
proto.aptos.transaction.testing1.v1.WriteTableItem.prototype.getKey = function() {
  return /** @type {string} */ (jspb.Message.getFieldWithDefault(this, 3, ""));
};


/**
 * @param {string} value
 * @return {!proto.aptos.transaction.testing1.v1.WriteTableItem} returns this
 */
proto.aptos.transaction.testing1.v1.WriteTableItem.prototype.setKey = function(value) {
  return jspb.Message.setProto3StringField(this, 3, value);
};


/**
 * optional WriteTableData data = 4;
 * @return {?proto.aptos.transaction.testing1.v1.WriteTableData}
 */
proto.aptos.transaction.testing1.v1.WriteTableItem.prototype.getData = function() {
  return /** @type{?proto.aptos.transaction.testing1.v1.WriteTableData} */ (
    jspb.Message.getWrapperField(this, proto.aptos.transaction.testing1.v1.WriteTableData, 4));
};


/**
 * @param {?proto.aptos.transaction.testing1.v1.WriteTableData|undefined} value
 * @return {!proto.aptos.transaction.testing1.v1.WriteTableItem} returns this
*/
proto.aptos.transaction.testing1.v1.WriteTableItem.prototype.setData = function(value) {
  return jspb.Message.setWrapperField(this, 4, value);
};


/**
 * Clears the message field making it undefined.
 * @return {!proto.aptos.transaction.testing1.v1.WriteTableItem} returns this
 */
proto.aptos.transaction.testing1.v1.WriteTableItem.prototype.clearData = function() {
  return this.setData(undefined);
};


/**
 * Returns whether this field is set.
 * @return {boolean}
 */
proto.aptos.transaction.testing1.v1.WriteTableItem.prototype.hasData = function() {
  return jspb.Message.getField(this, 4) != null;
};



/**
 * Oneof group definitions for this message. Each group defines the field
 * numbers belonging to that group. When of these fields' value is set, all
 * other fields in the group are cleared. During deserialization, if multiple
 * fields are encountered for a group, only the last value seen will be kept.
 * @private {!Array<!Array<number>>}
 * @const
 */
proto.aptos.transaction.testing1.v1.TransactionPayload.oneofGroups_ = [[2,3,4,5,6]];

/**
 * @enum {number}
 */
proto.aptos.transaction.testing1.v1.TransactionPayload.PayloadCase = {
  PAYLOAD_NOT_SET: 0,
  ENTRY_FUNCTION_PAYLOAD: 2,
  SCRIPT_PAYLOAD: 3,
  MODULE_BUNDLE_PAYLOAD: 4,
  WRITE_SET_PAYLOAD: 5,
  MULTISIG_PAYLOAD: 6
};

/**
 * @return {proto.aptos.transaction.testing1.v1.TransactionPayload.PayloadCase}
 */
proto.aptos.transaction.testing1.v1.TransactionPayload.prototype.getPayloadCase = function() {
  return /** @type {proto.aptos.transaction.testing1.v1.TransactionPayload.PayloadCase} */(jspb.Message.computeOneofCase(this, proto.aptos.transaction.testing1.v1.TransactionPayload.oneofGroups_[0]));
};



if (jspb.Message.GENERATE_TO_OBJECT) {
/**
 * Creates an object representation of this proto.
 * Field names that are reserved in JavaScript and will be renamed to pb_name.
 * Optional fields that are not set will be set to undefined.
 * To access a reserved field use, foo.pb_<name>, eg, foo.pb_default.
 * For the list of reserved names please see:
 *     net/proto2/compiler/js/internal/generator.cc#kKeyword.
 * @param {boolean=} opt_includeInstance Deprecated. whether to include the
 *     JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @return {!Object}
 */
proto.aptos.transaction.testing1.v1.TransactionPayload.prototype.toObject = function(opt_includeInstance) {
  return proto.aptos.transaction.testing1.v1.TransactionPayload.toObject(opt_includeInstance, this);
};


/**
 * Static version of the {@see toObject} method.
 * @param {boolean|undefined} includeInstance Deprecated. Whether to include
 *     the JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @param {!proto.aptos.transaction.testing1.v1.TransactionPayload} msg The msg instance to transform.
 * @return {!Object}
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.aptos.transaction.testing1.v1.TransactionPayload.toObject = function(includeInstance, msg) {
  var f, obj = {
    type: jspb.Message.getFieldWithDefault(msg, 1, 0),
    entryFunctionPayload: (f = msg.getEntryFunctionPayload()) && proto.aptos.transaction.testing1.v1.EntryFunctionPayload.toObject(includeInstance, f),
    scriptPayload: (f = msg.getScriptPayload()) && proto.aptos.transaction.testing1.v1.ScriptPayload.toObject(includeInstance, f),
    moduleBundlePayload: (f = msg.getModuleBundlePayload()) && proto.aptos.transaction.testing1.v1.ModuleBundlePayload.toObject(includeInstance, f),
    writeSetPayload: (f = msg.getWriteSetPayload()) && proto.aptos.transaction.testing1.v1.WriteSetPayload.toObject(includeInstance, f),
    multisigPayload: (f = msg.getMultisigPayload()) && proto.aptos.transaction.testing1.v1.MultisigPayload.toObject(includeInstance, f)
  };

  if (includeInstance) {
    obj.$jspbMessageInstance = msg;
  }
  return obj;
};
}


/**
 * Deserializes binary data (in protobuf wire format).
 * @param {jspb.ByteSource} bytes The bytes to deserialize.
 * @return {!proto.aptos.transaction.testing1.v1.TransactionPayload}
 */
proto.aptos.transaction.testing1.v1.TransactionPayload.deserializeBinary = function(bytes) {
  var reader = new jspb.BinaryReader(bytes);
  var msg = new proto.aptos.transaction.testing1.v1.TransactionPayload;
  return proto.aptos.transaction.testing1.v1.TransactionPayload.deserializeBinaryFromReader(msg, reader);
};


/**
 * Deserializes binary data (in protobuf wire format) from the
 * given reader into the given message object.
 * @param {!proto.aptos.transaction.testing1.v1.TransactionPayload} msg The message object to deserialize into.
 * @param {!jspb.BinaryReader} reader The BinaryReader to use.
 * @return {!proto.aptos.transaction.testing1.v1.TransactionPayload}
 */
proto.aptos.transaction.testing1.v1.TransactionPayload.deserializeBinaryFromReader = function(msg, reader) {
  while (reader.nextField()) {
    if (reader.isEndGroup()) {
      break;
    }
    var field = reader.getFieldNumber();
    switch (field) {
    case 1:
      var value = /** @type {!proto.aptos.transaction.testing1.v1.TransactionPayload.Type} */ (reader.readEnum());
      msg.setType(value);
      break;
    case 2:
      var value = new proto.aptos.transaction.testing1.v1.EntryFunctionPayload;
      reader.readMessage(value,proto.aptos.transaction.testing1.v1.EntryFunctionPayload.deserializeBinaryFromReader);
      msg.setEntryFunctionPayload(value);
      break;
    case 3:
      var value = new proto.aptos.transaction.testing1.v1.ScriptPayload;
      reader.readMessage(value,proto.aptos.transaction.testing1.v1.ScriptPayload.deserializeBinaryFromReader);
      msg.setScriptPayload(value);
      break;
    case 4:
      var value = new proto.aptos.transaction.testing1.v1.ModuleBundlePayload;
      reader.readMessage(value,proto.aptos.transaction.testing1.v1.ModuleBundlePayload.deserializeBinaryFromReader);
      msg.setModuleBundlePayload(value);
      break;
    case 5:
      var value = new proto.aptos.transaction.testing1.v1.WriteSetPayload;
      reader.readMessage(value,proto.aptos.transaction.testing1.v1.WriteSetPayload.deserializeBinaryFromReader);
      msg.setWriteSetPayload(value);
      break;
    case 6:
      var value = new proto.aptos.transaction.testing1.v1.MultisigPayload;
      reader.readMessage(value,proto.aptos.transaction.testing1.v1.MultisigPayload.deserializeBinaryFromReader);
      msg.setMultisigPayload(value);
      break;
    default:
      reader.skipField();
      break;
    }
  }
  return msg;
};


/**
 * Serializes the message to binary data (in protobuf wire format).
 * @return {!Uint8Array}
 */
proto.aptos.transaction.testing1.v1.TransactionPayload.prototype.serializeBinary = function() {
  var writer = new jspb.BinaryWriter();
  proto.aptos.transaction.testing1.v1.TransactionPayload.serializeBinaryToWriter(this, writer);
  return writer.getResultBuffer();
};


/**
 * Serializes the given message to binary data (in protobuf wire
 * format), writing to the given BinaryWriter.
 * @param {!proto.aptos.transaction.testing1.v1.TransactionPayload} message
 * @param {!jspb.BinaryWriter} writer
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.aptos.transaction.testing1.v1.TransactionPayload.serializeBinaryToWriter = function(message, writer) {
  var f = undefined;
  f = message.getType();
  if (f !== 0.0) {
    writer.writeEnum(
      1,
      f
    );
  }
  f = message.getEntryFunctionPayload();
  if (f != null) {
    writer.writeMessage(
      2,
      f,
      proto.aptos.transaction.testing1.v1.EntryFunctionPayload.serializeBinaryToWriter
    );
  }
  f = message.getScriptPayload();
  if (f != null) {
    writer.writeMessage(
      3,
      f,
      proto.aptos.transaction.testing1.v1.ScriptPayload.serializeBinaryToWriter
    );
  }
  f = message.getModuleBundlePayload();
  if (f != null) {
    writer.writeMessage(
      4,
      f,
      proto.aptos.transaction.testing1.v1.ModuleBundlePayload.serializeBinaryToWriter
    );
  }
  f = message.getWriteSetPayload();
  if (f != null) {
    writer.writeMessage(
      5,
      f,
      proto.aptos.transaction.testing1.v1.WriteSetPayload.serializeBinaryToWriter
    );
  }
  f = message.getMultisigPayload();
  if (f != null) {
    writer.writeMessage(
      6,
      f,
      proto.aptos.transaction.testing1.v1.MultisigPayload.serializeBinaryToWriter
    );
  }
};


/**
 * @enum {number}
 */
proto.aptos.transaction.testing1.v1.TransactionPayload.Type = {
  TYPE_UNSPECIFIED: 0,
  TYPE_ENTRY_FUNCTION_PAYLOAD: 1,
  TYPE_SCRIPT_PAYLOAD: 2,
  TYPE_MODULE_BUNDLE_PAYLOAD: 3,
  TYPE_WRITE_SET_PAYLOAD: 4,
  TYPE_MULTISIG_PAYLOAD: 5
};

/**
 * optional Type type = 1;
 * @return {!proto.aptos.transaction.testing1.v1.TransactionPayload.Type}
 */
proto.aptos.transaction.testing1.v1.TransactionPayload.prototype.getType = function() {
  return /** @type {!proto.aptos.transaction.testing1.v1.TransactionPayload.Type} */ (jspb.Message.getFieldWithDefault(this, 1, 0));
};


/**
 * @param {!proto.aptos.transaction.testing1.v1.TransactionPayload.Type} value
 * @return {!proto.aptos.transaction.testing1.v1.TransactionPayload} returns this
 */
proto.aptos.transaction.testing1.v1.TransactionPayload.prototype.setType = function(value) {
  return jspb.Message.setProto3EnumField(this, 1, value);
};


/**
 * optional EntryFunctionPayload entry_function_payload = 2;
 * @return {?proto.aptos.transaction.testing1.v1.EntryFunctionPayload}
 */
proto.aptos.transaction.testing1.v1.TransactionPayload.prototype.getEntryFunctionPayload = function() {
  return /** @type{?proto.aptos.transaction.testing1.v1.EntryFunctionPayload} */ (
    jspb.Message.getWrapperField(this, proto.aptos.transaction.testing1.v1.EntryFunctionPayload, 2));
};


/**
 * @param {?proto.aptos.transaction.testing1.v1.EntryFunctionPayload|undefined} value
 * @return {!proto.aptos.transaction.testing1.v1.TransactionPayload} returns this
*/
proto.aptos.transaction.testing1.v1.TransactionPayload.prototype.setEntryFunctionPayload = function(value) {
  return jspb.Message.setOneofWrapperField(this, 2, proto.aptos.transaction.testing1.v1.TransactionPayload.oneofGroups_[0], value);
};


/**
 * Clears the message field making it undefined.
 * @return {!proto.aptos.transaction.testing1.v1.TransactionPayload} returns this
 */
proto.aptos.transaction.testing1.v1.TransactionPayload.prototype.clearEntryFunctionPayload = function() {
  return this.setEntryFunctionPayload(undefined);
};


/**
 * Returns whether this field is set.
 * @return {boolean}
 */
proto.aptos.transaction.testing1.v1.TransactionPayload.prototype.hasEntryFunctionPayload = function() {
  return jspb.Message.getField(this, 2) != null;
};


/**
 * optional ScriptPayload script_payload = 3;
 * @return {?proto.aptos.transaction.testing1.v1.ScriptPayload}
 */
proto.aptos.transaction.testing1.v1.TransactionPayload.prototype.getScriptPayload = function() {
  return /** @type{?proto.aptos.transaction.testing1.v1.ScriptPayload} */ (
    jspb.Message.getWrapperField(this, proto.aptos.transaction.testing1.v1.ScriptPayload, 3));
};


/**
 * @param {?proto.aptos.transaction.testing1.v1.ScriptPayload|undefined} value
 * @return {!proto.aptos.transaction.testing1.v1.TransactionPayload} returns this
*/
proto.aptos.transaction.testing1.v1.TransactionPayload.prototype.setScriptPayload = function(value) {
  return jspb.Message.setOneofWrapperField(this, 3, proto.aptos.transaction.testing1.v1.TransactionPayload.oneofGroups_[0], value);
};


/**
 * Clears the message field making it undefined.
 * @return {!proto.aptos.transaction.testing1.v1.TransactionPayload} returns this
 */
proto.aptos.transaction.testing1.v1.TransactionPayload.prototype.clearScriptPayload = function() {
  return this.setScriptPayload(undefined);
};


/**
 * Returns whether this field is set.
 * @return {boolean}
 */
proto.aptos.transaction.testing1.v1.TransactionPayload.prototype.hasScriptPayload = function() {
  return jspb.Message.getField(this, 3) != null;
};


/**
 * optional ModuleBundlePayload module_bundle_payload = 4;
 * @return {?proto.aptos.transaction.testing1.v1.ModuleBundlePayload}
 */
proto.aptos.transaction.testing1.v1.TransactionPayload.prototype.getModuleBundlePayload = function() {
  return /** @type{?proto.aptos.transaction.testing1.v1.ModuleBundlePayload} */ (
    jspb.Message.getWrapperField(this, proto.aptos.transaction.testing1.v1.ModuleBundlePayload, 4));
};


/**
 * @param {?proto.aptos.transaction.testing1.v1.ModuleBundlePayload|undefined} value
 * @return {!proto.aptos.transaction.testing1.v1.TransactionPayload} returns this
*/
proto.aptos.transaction.testing1.v1.TransactionPayload.prototype.setModuleBundlePayload = function(value) {
  return jspb.Message.setOneofWrapperField(this, 4, proto.aptos.transaction.testing1.v1.TransactionPayload.oneofGroups_[0], value);
};


/**
 * Clears the message field making it undefined.
 * @return {!proto.aptos.transaction.testing1.v1.TransactionPayload} returns this
 */
proto.aptos.transaction.testing1.v1.TransactionPayload.prototype.clearModuleBundlePayload = function() {
  return this.setModuleBundlePayload(undefined);
};


/**
 * Returns whether this field is set.
 * @return {boolean}
 */
proto.aptos.transaction.testing1.v1.TransactionPayload.prototype.hasModuleBundlePayload = function() {
  return jspb.Message.getField(this, 4) != null;
};


/**
 * optional WriteSetPayload write_set_payload = 5;
 * @return {?proto.aptos.transaction.testing1.v1.WriteSetPayload}
 */
proto.aptos.transaction.testing1.v1.TransactionPayload.prototype.getWriteSetPayload = function() {
  return /** @type{?proto.aptos.transaction.testing1.v1.WriteSetPayload} */ (
    jspb.Message.getWrapperField(this, proto.aptos.transaction.testing1.v1.WriteSetPayload, 5));
};


/**
 * @param {?proto.aptos.transaction.testing1.v1.WriteSetPayload|undefined} value
 * @return {!proto.aptos.transaction.testing1.v1.TransactionPayload} returns this
*/
proto.aptos.transaction.testing1.v1.TransactionPayload.prototype.setWriteSetPayload = function(value) {
  return jspb.Message.setOneofWrapperField(this, 5, proto.aptos.transaction.testing1.v1.TransactionPayload.oneofGroups_[0], value);
};


/**
 * Clears the message field making it undefined.
 * @return {!proto.aptos.transaction.testing1.v1.TransactionPayload} returns this
 */
proto.aptos.transaction.testing1.v1.TransactionPayload.prototype.clearWriteSetPayload = function() {
  return this.setWriteSetPayload(undefined);
};


/**
 * Returns whether this field is set.
 * @return {boolean}
 */
proto.aptos.transaction.testing1.v1.TransactionPayload.prototype.hasWriteSetPayload = function() {
  return jspb.Message.getField(this, 5) != null;
};


/**
 * optional MultisigPayload multisig_payload = 6;
 * @return {?proto.aptos.transaction.testing1.v1.MultisigPayload}
 */
proto.aptos.transaction.testing1.v1.TransactionPayload.prototype.getMultisigPayload = function() {
  return /** @type{?proto.aptos.transaction.testing1.v1.MultisigPayload} */ (
    jspb.Message.getWrapperField(this, proto.aptos.transaction.testing1.v1.MultisigPayload, 6));
};


/**
 * @param {?proto.aptos.transaction.testing1.v1.MultisigPayload|undefined} value
 * @return {!proto.aptos.transaction.testing1.v1.TransactionPayload} returns this
*/
proto.aptos.transaction.testing1.v1.TransactionPayload.prototype.setMultisigPayload = function(value) {
  return jspb.Message.setOneofWrapperField(this, 6, proto.aptos.transaction.testing1.v1.TransactionPayload.oneofGroups_[0], value);
};


/**
 * Clears the message field making it undefined.
 * @return {!proto.aptos.transaction.testing1.v1.TransactionPayload} returns this
 */
proto.aptos.transaction.testing1.v1.TransactionPayload.prototype.clearMultisigPayload = function() {
  return this.setMultisigPayload(undefined);
};


/**
 * Returns whether this field is set.
 * @return {boolean}
 */
proto.aptos.transaction.testing1.v1.TransactionPayload.prototype.hasMultisigPayload = function() {
  return jspb.Message.getField(this, 6) != null;
};



/**
 * List of repeated fields within this message type.
 * @private {!Array<number>}
 * @const
 */
proto.aptos.transaction.testing1.v1.EntryFunctionPayload.repeatedFields_ = [2,3];



if (jspb.Message.GENERATE_TO_OBJECT) {
/**
 * Creates an object representation of this proto.
 * Field names that are reserved in JavaScript and will be renamed to pb_name.
 * Optional fields that are not set will be set to undefined.
 * To access a reserved field use, foo.pb_<name>, eg, foo.pb_default.
 * For the list of reserved names please see:
 *     net/proto2/compiler/js/internal/generator.cc#kKeyword.
 * @param {boolean=} opt_includeInstance Deprecated. whether to include the
 *     JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @return {!Object}
 */
proto.aptos.transaction.testing1.v1.EntryFunctionPayload.prototype.toObject = function(opt_includeInstance) {
  return proto.aptos.transaction.testing1.v1.EntryFunctionPayload.toObject(opt_includeInstance, this);
};


/**
 * Static version of the {@see toObject} method.
 * @param {boolean|undefined} includeInstance Deprecated. Whether to include
 *     the JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @param {!proto.aptos.transaction.testing1.v1.EntryFunctionPayload} msg The msg instance to transform.
 * @return {!Object}
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.aptos.transaction.testing1.v1.EntryFunctionPayload.toObject = function(includeInstance, msg) {
  var f, obj = {
    pb_function: (f = msg.getFunction()) && proto.aptos.transaction.testing1.v1.EntryFunctionId.toObject(includeInstance, f),
    typeArgumentsList: jspb.Message.toObjectList(msg.getTypeArgumentsList(),
    proto.aptos.transaction.testing1.v1.MoveType.toObject, includeInstance),
    argumentsList: (f = jspb.Message.getRepeatedField(msg, 3)) == null ? undefined : f,
    entryFunctionIdStr: jspb.Message.getFieldWithDefault(msg, 4, "")
  };

  if (includeInstance) {
    obj.$jspbMessageInstance = msg;
  }
  return obj;
};
}


/**
 * Deserializes binary data (in protobuf wire format).
 * @param {jspb.ByteSource} bytes The bytes to deserialize.
 * @return {!proto.aptos.transaction.testing1.v1.EntryFunctionPayload}
 */
proto.aptos.transaction.testing1.v1.EntryFunctionPayload.deserializeBinary = function(bytes) {
  var reader = new jspb.BinaryReader(bytes);
  var msg = new proto.aptos.transaction.testing1.v1.EntryFunctionPayload;
  return proto.aptos.transaction.testing1.v1.EntryFunctionPayload.deserializeBinaryFromReader(msg, reader);
};


/**
 * Deserializes binary data (in protobuf wire format) from the
 * given reader into the given message object.
 * @param {!proto.aptos.transaction.testing1.v1.EntryFunctionPayload} msg The message object to deserialize into.
 * @param {!jspb.BinaryReader} reader The BinaryReader to use.
 * @return {!proto.aptos.transaction.testing1.v1.EntryFunctionPayload}
 */
proto.aptos.transaction.testing1.v1.EntryFunctionPayload.deserializeBinaryFromReader = function(msg, reader) {
  while (reader.nextField()) {
    if (reader.isEndGroup()) {
      break;
    }
    var field = reader.getFieldNumber();
    switch (field) {
    case 1:
      var value = new proto.aptos.transaction.testing1.v1.EntryFunctionId;
      reader.readMessage(value,proto.aptos.transaction.testing1.v1.EntryFunctionId.deserializeBinaryFromReader);
      msg.setFunction(value);
      break;
    case 2:
      var value = new proto.aptos.transaction.testing1.v1.MoveType;
      reader.readMessage(value,proto.aptos.transaction.testing1.v1.MoveType.deserializeBinaryFromReader);
      msg.addTypeArguments(value);
      break;
    case 3:
      var value = /** @type {string} */ (reader.readString());
      msg.addArguments(value);
      break;
    case 4:
      var value = /** @type {string} */ (reader.readString());
      msg.setEntryFunctionIdStr(value);
      break;
    default:
      reader.skipField();
      break;
    }
  }
  return msg;
};


/**
 * Serializes the message to binary data (in protobuf wire format).
 * @return {!Uint8Array}
 */
proto.aptos.transaction.testing1.v1.EntryFunctionPayload.prototype.serializeBinary = function() {
  var writer = new jspb.BinaryWriter();
  proto.aptos.transaction.testing1.v1.EntryFunctionPayload.serializeBinaryToWriter(this, writer);
  return writer.getResultBuffer();
};


/**
 * Serializes the given message to binary data (in protobuf wire
 * format), writing to the given BinaryWriter.
 * @param {!proto.aptos.transaction.testing1.v1.EntryFunctionPayload} message
 * @param {!jspb.BinaryWriter} writer
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.aptos.transaction.testing1.v1.EntryFunctionPayload.serializeBinaryToWriter = function(message, writer) {
  var f = undefined;
  f = message.getFunction();
  if (f != null) {
    writer.writeMessage(
      1,
      f,
      proto.aptos.transaction.testing1.v1.EntryFunctionId.serializeBinaryToWriter
    );
  }
  f = message.getTypeArgumentsList();
  if (f.length > 0) {
    writer.writeRepeatedMessage(
      2,
      f,
      proto.aptos.transaction.testing1.v1.MoveType.serializeBinaryToWriter
    );
  }
  f = message.getArgumentsList();
  if (f.length > 0) {
    writer.writeRepeatedString(
      3,
      f
    );
  }
  f = message.getEntryFunctionIdStr();
  if (f.length > 0) {
    writer.writeString(
      4,
      f
    );
  }
};


/**
 * optional EntryFunctionId function = 1;
 * @return {?proto.aptos.transaction.testing1.v1.EntryFunctionId}
 */
proto.aptos.transaction.testing1.v1.EntryFunctionPayload.prototype.getFunction = function() {
  return /** @type{?proto.aptos.transaction.testing1.v1.EntryFunctionId} */ (
    jspb.Message.getWrapperField(this, proto.aptos.transaction.testing1.v1.EntryFunctionId, 1));
};


/**
 * @param {?proto.aptos.transaction.testing1.v1.EntryFunctionId|undefined} value
 * @return {!proto.aptos.transaction.testing1.v1.EntryFunctionPayload} returns this
*/
proto.aptos.transaction.testing1.v1.EntryFunctionPayload.prototype.setFunction = function(value) {
  return jspb.Message.setWrapperField(this, 1, value);
};


/**
 * Clears the message field making it undefined.
 * @return {!proto.aptos.transaction.testing1.v1.EntryFunctionPayload} returns this
 */
proto.aptos.transaction.testing1.v1.EntryFunctionPayload.prototype.clearFunction = function() {
  return this.setFunction(undefined);
};


/**
 * Returns whether this field is set.
 * @return {boolean}
 */
proto.aptos.transaction.testing1.v1.EntryFunctionPayload.prototype.hasFunction = function() {
  return jspb.Message.getField(this, 1) != null;
};


/**
 * repeated MoveType type_arguments = 2;
 * @return {!Array<!proto.aptos.transaction.testing1.v1.MoveType>}
 */
proto.aptos.transaction.testing1.v1.EntryFunctionPayload.prototype.getTypeArgumentsList = function() {
  return /** @type{!Array<!proto.aptos.transaction.testing1.v1.MoveType>} */ (
    jspb.Message.getRepeatedWrapperField(this, proto.aptos.transaction.testing1.v1.MoveType, 2));
};


/**
 * @param {!Array<!proto.aptos.transaction.testing1.v1.MoveType>} value
 * @return {!proto.aptos.transaction.testing1.v1.EntryFunctionPayload} returns this
*/
proto.aptos.transaction.testing1.v1.EntryFunctionPayload.prototype.setTypeArgumentsList = function(value) {
  return jspb.Message.setRepeatedWrapperField(this, 2, value);
};


/**
 * @param {!proto.aptos.transaction.testing1.v1.MoveType=} opt_value
 * @param {number=} opt_index
 * @return {!proto.aptos.transaction.testing1.v1.MoveType}
 */
proto.aptos.transaction.testing1.v1.EntryFunctionPayload.prototype.addTypeArguments = function(opt_value, opt_index) {
  return jspb.Message.addToRepeatedWrapperField(this, 2, opt_value, proto.aptos.transaction.testing1.v1.MoveType, opt_index);
};


/**
 * Clears the list making it empty but non-null.
 * @return {!proto.aptos.transaction.testing1.v1.EntryFunctionPayload} returns this
 */
proto.aptos.transaction.testing1.v1.EntryFunctionPayload.prototype.clearTypeArgumentsList = function() {
  return this.setTypeArgumentsList([]);
};


/**
 * repeated string arguments = 3;
 * @return {!Array<string>}
 */
proto.aptos.transaction.testing1.v1.EntryFunctionPayload.prototype.getArgumentsList = function() {
  return /** @type {!Array<string>} */ (jspb.Message.getRepeatedField(this, 3));
};


/**
 * @param {!Array<string>} value
 * @return {!proto.aptos.transaction.testing1.v1.EntryFunctionPayload} returns this
 */
proto.aptos.transaction.testing1.v1.EntryFunctionPayload.prototype.setArgumentsList = function(value) {
  return jspb.Message.setField(this, 3, value || []);
};


/**
 * @param {string} value
 * @param {number=} opt_index
 * @return {!proto.aptos.transaction.testing1.v1.EntryFunctionPayload} returns this
 */
proto.aptos.transaction.testing1.v1.EntryFunctionPayload.prototype.addArguments = function(value, opt_index) {
  return jspb.Message.addToRepeatedField(this, 3, value, opt_index);
};


/**
 * Clears the list making it empty but non-null.
 * @return {!proto.aptos.transaction.testing1.v1.EntryFunctionPayload} returns this
 */
proto.aptos.transaction.testing1.v1.EntryFunctionPayload.prototype.clearArgumentsList = function() {
  return this.setArgumentsList([]);
};


/**
 * optional string entry_function_id_str = 4;
 * @return {string}
 */
proto.aptos.transaction.testing1.v1.EntryFunctionPayload.prototype.getEntryFunctionIdStr = function() {
  return /** @type {string} */ (jspb.Message.getFieldWithDefault(this, 4, ""));
};


/**
 * @param {string} value
 * @return {!proto.aptos.transaction.testing1.v1.EntryFunctionPayload} returns this
 */
proto.aptos.transaction.testing1.v1.EntryFunctionPayload.prototype.setEntryFunctionIdStr = function(value) {
  return jspb.Message.setProto3StringField(this, 4, value);
};





if (jspb.Message.GENERATE_TO_OBJECT) {
/**
 * Creates an object representation of this proto.
 * Field names that are reserved in JavaScript and will be renamed to pb_name.
 * Optional fields that are not set will be set to undefined.
 * To access a reserved field use, foo.pb_<name>, eg, foo.pb_default.
 * For the list of reserved names please see:
 *     net/proto2/compiler/js/internal/generator.cc#kKeyword.
 * @param {boolean=} opt_includeInstance Deprecated. whether to include the
 *     JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @return {!Object}
 */
proto.aptos.transaction.testing1.v1.MoveScriptBytecode.prototype.toObject = function(opt_includeInstance) {
  return proto.aptos.transaction.testing1.v1.MoveScriptBytecode.toObject(opt_includeInstance, this);
};


/**
 * Static version of the {@see toObject} method.
 * @param {boolean|undefined} includeInstance Deprecated. Whether to include
 *     the JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @param {!proto.aptos.transaction.testing1.v1.MoveScriptBytecode} msg The msg instance to transform.
 * @return {!Object}
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.aptos.transaction.testing1.v1.MoveScriptBytecode.toObject = function(includeInstance, msg) {
  var f, obj = {
    bytecode: msg.getBytecode_asB64(),
    abi: (f = msg.getAbi()) && proto.aptos.transaction.testing1.v1.MoveFunction.toObject(includeInstance, f)
  };

  if (includeInstance) {
    obj.$jspbMessageInstance = msg;
  }
  return obj;
};
}


/**
 * Deserializes binary data (in protobuf wire format).
 * @param {jspb.ByteSource} bytes The bytes to deserialize.
 * @return {!proto.aptos.transaction.testing1.v1.MoveScriptBytecode}
 */
proto.aptos.transaction.testing1.v1.MoveScriptBytecode.deserializeBinary = function(bytes) {
  var reader = new jspb.BinaryReader(bytes);
  var msg = new proto.aptos.transaction.testing1.v1.MoveScriptBytecode;
  return proto.aptos.transaction.testing1.v1.MoveScriptBytecode.deserializeBinaryFromReader(msg, reader);
};


/**
 * Deserializes binary data (in protobuf wire format) from the
 * given reader into the given message object.
 * @param {!proto.aptos.transaction.testing1.v1.MoveScriptBytecode} msg The message object to deserialize into.
 * @param {!jspb.BinaryReader} reader The BinaryReader to use.
 * @return {!proto.aptos.transaction.testing1.v1.MoveScriptBytecode}
 */
proto.aptos.transaction.testing1.v1.MoveScriptBytecode.deserializeBinaryFromReader = function(msg, reader) {
  while (reader.nextField()) {
    if (reader.isEndGroup()) {
      break;
    }
    var field = reader.getFieldNumber();
    switch (field) {
    case 1:
      var value = /** @type {!Uint8Array} */ (reader.readBytes());
      msg.setBytecode(value);
      break;
    case 2:
      var value = new proto.aptos.transaction.testing1.v1.MoveFunction;
      reader.readMessage(value,proto.aptos.transaction.testing1.v1.MoveFunction.deserializeBinaryFromReader);
      msg.setAbi(value);
      break;
    default:
      reader.skipField();
      break;
    }
  }
  return msg;
};


/**
 * Serializes the message to binary data (in protobuf wire format).
 * @return {!Uint8Array}
 */
proto.aptos.transaction.testing1.v1.MoveScriptBytecode.prototype.serializeBinary = function() {
  var writer = new jspb.BinaryWriter();
  proto.aptos.transaction.testing1.v1.MoveScriptBytecode.serializeBinaryToWriter(this, writer);
  return writer.getResultBuffer();
};


/**
 * Serializes the given message to binary data (in protobuf wire
 * format), writing to the given BinaryWriter.
 * @param {!proto.aptos.transaction.testing1.v1.MoveScriptBytecode} message
 * @param {!jspb.BinaryWriter} writer
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.aptos.transaction.testing1.v1.MoveScriptBytecode.serializeBinaryToWriter = function(message, writer) {
  var f = undefined;
  f = message.getBytecode_asU8();
  if (f.length > 0) {
    writer.writeBytes(
      1,
      f
    );
  }
  f = message.getAbi();
  if (f != null) {
    writer.writeMessage(
      2,
      f,
      proto.aptos.transaction.testing1.v1.MoveFunction.serializeBinaryToWriter
    );
  }
};


/**
 * optional bytes bytecode = 1;
 * @return {!(string|Uint8Array)}
 */
proto.aptos.transaction.testing1.v1.MoveScriptBytecode.prototype.getBytecode = function() {
  return /** @type {!(string|Uint8Array)} */ (jspb.Message.getFieldWithDefault(this, 1, ""));
};


/**
 * optional bytes bytecode = 1;
 * This is a type-conversion wrapper around `getBytecode()`
 * @return {string}
 */
proto.aptos.transaction.testing1.v1.MoveScriptBytecode.prototype.getBytecode_asB64 = function() {
  return /** @type {string} */ (jspb.Message.bytesAsB64(
      this.getBytecode()));
};


/**
 * optional bytes bytecode = 1;
 * Note that Uint8Array is not supported on all browsers.
 * @see http://caniuse.com/Uint8Array
 * This is a type-conversion wrapper around `getBytecode()`
 * @return {!Uint8Array}
 */
proto.aptos.transaction.testing1.v1.MoveScriptBytecode.prototype.getBytecode_asU8 = function() {
  return /** @type {!Uint8Array} */ (jspb.Message.bytesAsU8(
      this.getBytecode()));
};


/**
 * @param {!(string|Uint8Array)} value
 * @return {!proto.aptos.transaction.testing1.v1.MoveScriptBytecode} returns this
 */
proto.aptos.transaction.testing1.v1.MoveScriptBytecode.prototype.setBytecode = function(value) {
  return jspb.Message.setProto3BytesField(this, 1, value);
};


/**
 * optional MoveFunction abi = 2;
 * @return {?proto.aptos.transaction.testing1.v1.MoveFunction}
 */
proto.aptos.transaction.testing1.v1.MoveScriptBytecode.prototype.getAbi = function() {
  return /** @type{?proto.aptos.transaction.testing1.v1.MoveFunction} */ (
    jspb.Message.getWrapperField(this, proto.aptos.transaction.testing1.v1.MoveFunction, 2));
};


/**
 * @param {?proto.aptos.transaction.testing1.v1.MoveFunction|undefined} value
 * @return {!proto.aptos.transaction.testing1.v1.MoveScriptBytecode} returns this
*/
proto.aptos.transaction.testing1.v1.MoveScriptBytecode.prototype.setAbi = function(value) {
  return jspb.Message.setWrapperField(this, 2, value);
};


/**
 * Clears the message field making it undefined.
 * @return {!proto.aptos.transaction.testing1.v1.MoveScriptBytecode} returns this
 */
proto.aptos.transaction.testing1.v1.MoveScriptBytecode.prototype.clearAbi = function() {
  return this.setAbi(undefined);
};


/**
 * Returns whether this field is set.
 * @return {boolean}
 */
proto.aptos.transaction.testing1.v1.MoveScriptBytecode.prototype.hasAbi = function() {
  return jspb.Message.getField(this, 2) != null;
};



/**
 * List of repeated fields within this message type.
 * @private {!Array<number>}
 * @const
 */
proto.aptos.transaction.testing1.v1.ScriptPayload.repeatedFields_ = [2,3];



if (jspb.Message.GENERATE_TO_OBJECT) {
/**
 * Creates an object representation of this proto.
 * Field names that are reserved in JavaScript and will be renamed to pb_name.
 * Optional fields that are not set will be set to undefined.
 * To access a reserved field use, foo.pb_<name>, eg, foo.pb_default.
 * For the list of reserved names please see:
 *     net/proto2/compiler/js/internal/generator.cc#kKeyword.
 * @param {boolean=} opt_includeInstance Deprecated. whether to include the
 *     JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @return {!Object}
 */
proto.aptos.transaction.testing1.v1.ScriptPayload.prototype.toObject = function(opt_includeInstance) {
  return proto.aptos.transaction.testing1.v1.ScriptPayload.toObject(opt_includeInstance, this);
};


/**
 * Static version of the {@see toObject} method.
 * @param {boolean|undefined} includeInstance Deprecated. Whether to include
 *     the JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @param {!proto.aptos.transaction.testing1.v1.ScriptPayload} msg The msg instance to transform.
 * @return {!Object}
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.aptos.transaction.testing1.v1.ScriptPayload.toObject = function(includeInstance, msg) {
  var f, obj = {
    code: (f = msg.getCode()) && proto.aptos.transaction.testing1.v1.MoveScriptBytecode.toObject(includeInstance, f),
    typeArgumentsList: jspb.Message.toObjectList(msg.getTypeArgumentsList(),
    proto.aptos.transaction.testing1.v1.MoveType.toObject, includeInstance),
    argumentsList: (f = jspb.Message.getRepeatedField(msg, 3)) == null ? undefined : f
  };

  if (includeInstance) {
    obj.$jspbMessageInstance = msg;
  }
  return obj;
};
}


/**
 * Deserializes binary data (in protobuf wire format).
 * @param {jspb.ByteSource} bytes The bytes to deserialize.
 * @return {!proto.aptos.transaction.testing1.v1.ScriptPayload}
 */
proto.aptos.transaction.testing1.v1.ScriptPayload.deserializeBinary = function(bytes) {
  var reader = new jspb.BinaryReader(bytes);
  var msg = new proto.aptos.transaction.testing1.v1.ScriptPayload;
  return proto.aptos.transaction.testing1.v1.ScriptPayload.deserializeBinaryFromReader(msg, reader);
};


/**
 * Deserializes binary data (in protobuf wire format) from the
 * given reader into the given message object.
 * @param {!proto.aptos.transaction.testing1.v1.ScriptPayload} msg The message object to deserialize into.
 * @param {!jspb.BinaryReader} reader The BinaryReader to use.
 * @return {!proto.aptos.transaction.testing1.v1.ScriptPayload}
 */
proto.aptos.transaction.testing1.v1.ScriptPayload.deserializeBinaryFromReader = function(msg, reader) {
  while (reader.nextField()) {
    if (reader.isEndGroup()) {
      break;
    }
    var field = reader.getFieldNumber();
    switch (field) {
    case 1:
      var value = new proto.aptos.transaction.testing1.v1.MoveScriptBytecode;
      reader.readMessage(value,proto.aptos.transaction.testing1.v1.MoveScriptBytecode.deserializeBinaryFromReader);
      msg.setCode(value);
      break;
    case 2:
      var value = new proto.aptos.transaction.testing1.v1.MoveType;
      reader.readMessage(value,proto.aptos.transaction.testing1.v1.MoveType.deserializeBinaryFromReader);
      msg.addTypeArguments(value);
      break;
    case 3:
      var value = /** @type {string} */ (reader.readString());
      msg.addArguments(value);
      break;
    default:
      reader.skipField();
      break;
    }
  }
  return msg;
};


/**
 * Serializes the message to binary data (in protobuf wire format).
 * @return {!Uint8Array}
 */
proto.aptos.transaction.testing1.v1.ScriptPayload.prototype.serializeBinary = function() {
  var writer = new jspb.BinaryWriter();
  proto.aptos.transaction.testing1.v1.ScriptPayload.serializeBinaryToWriter(this, writer);
  return writer.getResultBuffer();
};


/**
 * Serializes the given message to binary data (in protobuf wire
 * format), writing to the given BinaryWriter.
 * @param {!proto.aptos.transaction.testing1.v1.ScriptPayload} message
 * @param {!jspb.BinaryWriter} writer
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.aptos.transaction.testing1.v1.ScriptPayload.serializeBinaryToWriter = function(message, writer) {
  var f = undefined;
  f = message.getCode();
  if (f != null) {
    writer.writeMessage(
      1,
      f,
      proto.aptos.transaction.testing1.v1.MoveScriptBytecode.serializeBinaryToWriter
    );
  }
  f = message.getTypeArgumentsList();
  if (f.length > 0) {
    writer.writeRepeatedMessage(
      2,
      f,
      proto.aptos.transaction.testing1.v1.MoveType.serializeBinaryToWriter
    );
  }
  f = message.getArgumentsList();
  if (f.length > 0) {
    writer.writeRepeatedString(
      3,
      f
    );
  }
};


/**
 * optional MoveScriptBytecode code = 1;
 * @return {?proto.aptos.transaction.testing1.v1.MoveScriptBytecode}
 */
proto.aptos.transaction.testing1.v1.ScriptPayload.prototype.getCode = function() {
  return /** @type{?proto.aptos.transaction.testing1.v1.MoveScriptBytecode} */ (
    jspb.Message.getWrapperField(this, proto.aptos.transaction.testing1.v1.MoveScriptBytecode, 1));
};


/**
 * @param {?proto.aptos.transaction.testing1.v1.MoveScriptBytecode|undefined} value
 * @return {!proto.aptos.transaction.testing1.v1.ScriptPayload} returns this
*/
proto.aptos.transaction.testing1.v1.ScriptPayload.prototype.setCode = function(value) {
  return jspb.Message.setWrapperField(this, 1, value);
};


/**
 * Clears the message field making it undefined.
 * @return {!proto.aptos.transaction.testing1.v1.ScriptPayload} returns this
 */
proto.aptos.transaction.testing1.v1.ScriptPayload.prototype.clearCode = function() {
  return this.setCode(undefined);
};


/**
 * Returns whether this field is set.
 * @return {boolean}
 */
proto.aptos.transaction.testing1.v1.ScriptPayload.prototype.hasCode = function() {
  return jspb.Message.getField(this, 1) != null;
};


/**
 * repeated MoveType type_arguments = 2;
 * @return {!Array<!proto.aptos.transaction.testing1.v1.MoveType>}
 */
proto.aptos.transaction.testing1.v1.ScriptPayload.prototype.getTypeArgumentsList = function() {
  return /** @type{!Array<!proto.aptos.transaction.testing1.v1.MoveType>} */ (
    jspb.Message.getRepeatedWrapperField(this, proto.aptos.transaction.testing1.v1.MoveType, 2));
};


/**
 * @param {!Array<!proto.aptos.transaction.testing1.v1.MoveType>} value
 * @return {!proto.aptos.transaction.testing1.v1.ScriptPayload} returns this
*/
proto.aptos.transaction.testing1.v1.ScriptPayload.prototype.setTypeArgumentsList = function(value) {
  return jspb.Message.setRepeatedWrapperField(this, 2, value);
};


/**
 * @param {!proto.aptos.transaction.testing1.v1.MoveType=} opt_value
 * @param {number=} opt_index
 * @return {!proto.aptos.transaction.testing1.v1.MoveType}
 */
proto.aptos.transaction.testing1.v1.ScriptPayload.prototype.addTypeArguments = function(opt_value, opt_index) {
  return jspb.Message.addToRepeatedWrapperField(this, 2, opt_value, proto.aptos.transaction.testing1.v1.MoveType, opt_index);
};


/**
 * Clears the list making it empty but non-null.
 * @return {!proto.aptos.transaction.testing1.v1.ScriptPayload} returns this
 */
proto.aptos.transaction.testing1.v1.ScriptPayload.prototype.clearTypeArgumentsList = function() {
  return this.setTypeArgumentsList([]);
};


/**
 * repeated string arguments = 3;
 * @return {!Array<string>}
 */
proto.aptos.transaction.testing1.v1.ScriptPayload.prototype.getArgumentsList = function() {
  return /** @type {!Array<string>} */ (jspb.Message.getRepeatedField(this, 3));
};


/**
 * @param {!Array<string>} value
 * @return {!proto.aptos.transaction.testing1.v1.ScriptPayload} returns this
 */
proto.aptos.transaction.testing1.v1.ScriptPayload.prototype.setArgumentsList = function(value) {
  return jspb.Message.setField(this, 3, value || []);
};


/**
 * @param {string} value
 * @param {number=} opt_index
 * @return {!proto.aptos.transaction.testing1.v1.ScriptPayload} returns this
 */
proto.aptos.transaction.testing1.v1.ScriptPayload.prototype.addArguments = function(value, opt_index) {
  return jspb.Message.addToRepeatedField(this, 3, value, opt_index);
};


/**
 * Clears the list making it empty but non-null.
 * @return {!proto.aptos.transaction.testing1.v1.ScriptPayload} returns this
 */
proto.aptos.transaction.testing1.v1.ScriptPayload.prototype.clearArgumentsList = function() {
  return this.setArgumentsList([]);
};





if (jspb.Message.GENERATE_TO_OBJECT) {
/**
 * Creates an object representation of this proto.
 * Field names that are reserved in JavaScript and will be renamed to pb_name.
 * Optional fields that are not set will be set to undefined.
 * To access a reserved field use, foo.pb_<name>, eg, foo.pb_default.
 * For the list of reserved names please see:
 *     net/proto2/compiler/js/internal/generator.cc#kKeyword.
 * @param {boolean=} opt_includeInstance Deprecated. whether to include the
 *     JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @return {!Object}
 */
proto.aptos.transaction.testing1.v1.MultisigPayload.prototype.toObject = function(opt_includeInstance) {
  return proto.aptos.transaction.testing1.v1.MultisigPayload.toObject(opt_includeInstance, this);
};


/**
 * Static version of the {@see toObject} method.
 * @param {boolean|undefined} includeInstance Deprecated. Whether to include
 *     the JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @param {!proto.aptos.transaction.testing1.v1.MultisigPayload} msg The msg instance to transform.
 * @return {!Object}
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.aptos.transaction.testing1.v1.MultisigPayload.toObject = function(includeInstance, msg) {
  var f, obj = {
    multisigAddress: jspb.Message.getFieldWithDefault(msg, 1, ""),
    transactionPayload: (f = msg.getTransactionPayload()) && proto.aptos.transaction.testing1.v1.MultisigTransactionPayload.toObject(includeInstance, f)
  };

  if (includeInstance) {
    obj.$jspbMessageInstance = msg;
  }
  return obj;
};
}


/**
 * Deserializes binary data (in protobuf wire format).
 * @param {jspb.ByteSource} bytes The bytes to deserialize.
 * @return {!proto.aptos.transaction.testing1.v1.MultisigPayload}
 */
proto.aptos.transaction.testing1.v1.MultisigPayload.deserializeBinary = function(bytes) {
  var reader = new jspb.BinaryReader(bytes);
  var msg = new proto.aptos.transaction.testing1.v1.MultisigPayload;
  return proto.aptos.transaction.testing1.v1.MultisigPayload.deserializeBinaryFromReader(msg, reader);
};


/**
 * Deserializes binary data (in protobuf wire format) from the
 * given reader into the given message object.
 * @param {!proto.aptos.transaction.testing1.v1.MultisigPayload} msg The message object to deserialize into.
 * @param {!jspb.BinaryReader} reader The BinaryReader to use.
 * @return {!proto.aptos.transaction.testing1.v1.MultisigPayload}
 */
proto.aptos.transaction.testing1.v1.MultisigPayload.deserializeBinaryFromReader = function(msg, reader) {
  while (reader.nextField()) {
    if (reader.isEndGroup()) {
      break;
    }
    var field = reader.getFieldNumber();
    switch (field) {
    case 1:
      var value = /** @type {string} */ (reader.readString());
      msg.setMultisigAddress(value);
      break;
    case 2:
      var value = new proto.aptos.transaction.testing1.v1.MultisigTransactionPayload;
      reader.readMessage(value,proto.aptos.transaction.testing1.v1.MultisigTransactionPayload.deserializeBinaryFromReader);
      msg.setTransactionPayload(value);
      break;
    default:
      reader.skipField();
      break;
    }
  }
  return msg;
};


/**
 * Serializes the message to binary data (in protobuf wire format).
 * @return {!Uint8Array}
 */
proto.aptos.transaction.testing1.v1.MultisigPayload.prototype.serializeBinary = function() {
  var writer = new jspb.BinaryWriter();
  proto.aptos.transaction.testing1.v1.MultisigPayload.serializeBinaryToWriter(this, writer);
  return writer.getResultBuffer();
};


/**
 * Serializes the given message to binary data (in protobuf wire
 * format), writing to the given BinaryWriter.
 * @param {!proto.aptos.transaction.testing1.v1.MultisigPayload} message
 * @param {!jspb.BinaryWriter} writer
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.aptos.transaction.testing1.v1.MultisigPayload.serializeBinaryToWriter = function(message, writer) {
  var f = undefined;
  f = message.getMultisigAddress();
  if (f.length > 0) {
    writer.writeString(
      1,
      f
    );
  }
  f = message.getTransactionPayload();
  if (f != null) {
    writer.writeMessage(
      2,
      f,
      proto.aptos.transaction.testing1.v1.MultisigTransactionPayload.serializeBinaryToWriter
    );
  }
};


/**
 * optional string multisig_address = 1;
 * @return {string}
 */
proto.aptos.transaction.testing1.v1.MultisigPayload.prototype.getMultisigAddress = function() {
  return /** @type {string} */ (jspb.Message.getFieldWithDefault(this, 1, ""));
};


/**
 * @param {string} value
 * @return {!proto.aptos.transaction.testing1.v1.MultisigPayload} returns this
 */
proto.aptos.transaction.testing1.v1.MultisigPayload.prototype.setMultisigAddress = function(value) {
  return jspb.Message.setProto3StringField(this, 1, value);
};


/**
 * optional MultisigTransactionPayload transaction_payload = 2;
 * @return {?proto.aptos.transaction.testing1.v1.MultisigTransactionPayload}
 */
proto.aptos.transaction.testing1.v1.MultisigPayload.prototype.getTransactionPayload = function() {
  return /** @type{?proto.aptos.transaction.testing1.v1.MultisigTransactionPayload} */ (
    jspb.Message.getWrapperField(this, proto.aptos.transaction.testing1.v1.MultisigTransactionPayload, 2));
};


/**
 * @param {?proto.aptos.transaction.testing1.v1.MultisigTransactionPayload|undefined} value
 * @return {!proto.aptos.transaction.testing1.v1.MultisigPayload} returns this
*/
proto.aptos.transaction.testing1.v1.MultisigPayload.prototype.setTransactionPayload = function(value) {
  return jspb.Message.setWrapperField(this, 2, value);
};


/**
 * Clears the message field making it undefined.
 * @return {!proto.aptos.transaction.testing1.v1.MultisigPayload} returns this
 */
proto.aptos.transaction.testing1.v1.MultisigPayload.prototype.clearTransactionPayload = function() {
  return this.setTransactionPayload(undefined);
};


/**
 * Returns whether this field is set.
 * @return {boolean}
 */
proto.aptos.transaction.testing1.v1.MultisigPayload.prototype.hasTransactionPayload = function() {
  return jspb.Message.getField(this, 2) != null;
};



/**
 * Oneof group definitions for this message. Each group defines the field
 * numbers belonging to that group. When of these fields' value is set, all
 * other fields in the group are cleared. During deserialization, if multiple
 * fields are encountered for a group, only the last value seen will be kept.
 * @private {!Array<!Array<number>>}
 * @const
 */
proto.aptos.transaction.testing1.v1.MultisigTransactionPayload.oneofGroups_ = [[2]];

/**
 * @enum {number}
 */
proto.aptos.transaction.testing1.v1.MultisigTransactionPayload.PayloadCase = {
  PAYLOAD_NOT_SET: 0,
  ENTRY_FUNCTION_PAYLOAD: 2
};

/**
 * @return {proto.aptos.transaction.testing1.v1.MultisigTransactionPayload.PayloadCase}
 */
proto.aptos.transaction.testing1.v1.MultisigTransactionPayload.prototype.getPayloadCase = function() {
  return /** @type {proto.aptos.transaction.testing1.v1.MultisigTransactionPayload.PayloadCase} */(jspb.Message.computeOneofCase(this, proto.aptos.transaction.testing1.v1.MultisigTransactionPayload.oneofGroups_[0]));
};



if (jspb.Message.GENERATE_TO_OBJECT) {
/**
 * Creates an object representation of this proto.
 * Field names that are reserved in JavaScript and will be renamed to pb_name.
 * Optional fields that are not set will be set to undefined.
 * To access a reserved field use, foo.pb_<name>, eg, foo.pb_default.
 * For the list of reserved names please see:
 *     net/proto2/compiler/js/internal/generator.cc#kKeyword.
 * @param {boolean=} opt_includeInstance Deprecated. whether to include the
 *     JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @return {!Object}
 */
proto.aptos.transaction.testing1.v1.MultisigTransactionPayload.prototype.toObject = function(opt_includeInstance) {
  return proto.aptos.transaction.testing1.v1.MultisigTransactionPayload.toObject(opt_includeInstance, this);
};


/**
 * Static version of the {@see toObject} method.
 * @param {boolean|undefined} includeInstance Deprecated. Whether to include
 *     the JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @param {!proto.aptos.transaction.testing1.v1.MultisigTransactionPayload} msg The msg instance to transform.
 * @return {!Object}
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.aptos.transaction.testing1.v1.MultisigTransactionPayload.toObject = function(includeInstance, msg) {
  var f, obj = {
    type: jspb.Message.getFieldWithDefault(msg, 1, 0),
    entryFunctionPayload: (f = msg.getEntryFunctionPayload()) && proto.aptos.transaction.testing1.v1.EntryFunctionPayload.toObject(includeInstance, f)
  };

  if (includeInstance) {
    obj.$jspbMessageInstance = msg;
  }
  return obj;
};
}


/**
 * Deserializes binary data (in protobuf wire format).
 * @param {jspb.ByteSource} bytes The bytes to deserialize.
 * @return {!proto.aptos.transaction.testing1.v1.MultisigTransactionPayload}
 */
proto.aptos.transaction.testing1.v1.MultisigTransactionPayload.deserializeBinary = function(bytes) {
  var reader = new jspb.BinaryReader(bytes);
  var msg = new proto.aptos.transaction.testing1.v1.MultisigTransactionPayload;
  return proto.aptos.transaction.testing1.v1.MultisigTransactionPayload.deserializeBinaryFromReader(msg, reader);
};


/**
 * Deserializes binary data (in protobuf wire format) from the
 * given reader into the given message object.
 * @param {!proto.aptos.transaction.testing1.v1.MultisigTransactionPayload} msg The message object to deserialize into.
 * @param {!jspb.BinaryReader} reader The BinaryReader to use.
 * @return {!proto.aptos.transaction.testing1.v1.MultisigTransactionPayload}
 */
proto.aptos.transaction.testing1.v1.MultisigTransactionPayload.deserializeBinaryFromReader = function(msg, reader) {
  while (reader.nextField()) {
    if (reader.isEndGroup()) {
      break;
    }
    var field = reader.getFieldNumber();
    switch (field) {
    case 1:
      var value = /** @type {!proto.aptos.transaction.testing1.v1.MultisigTransactionPayload.Type} */ (reader.readEnum());
      msg.setType(value);
      break;
    case 2:
      var value = new proto.aptos.transaction.testing1.v1.EntryFunctionPayload;
      reader.readMessage(value,proto.aptos.transaction.testing1.v1.EntryFunctionPayload.deserializeBinaryFromReader);
      msg.setEntryFunctionPayload(value);
      break;
    default:
      reader.skipField();
      break;
    }
  }
  return msg;
};


/**
 * Serializes the message to binary data (in protobuf wire format).
 * @return {!Uint8Array}
 */
proto.aptos.transaction.testing1.v1.MultisigTransactionPayload.prototype.serializeBinary = function() {
  var writer = new jspb.BinaryWriter();
  proto.aptos.transaction.testing1.v1.MultisigTransactionPayload.serializeBinaryToWriter(this, writer);
  return writer.getResultBuffer();
};


/**
 * Serializes the given message to binary data (in protobuf wire
 * format), writing to the given BinaryWriter.
 * @param {!proto.aptos.transaction.testing1.v1.MultisigTransactionPayload} message
 * @param {!jspb.BinaryWriter} writer
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.aptos.transaction.testing1.v1.MultisigTransactionPayload.serializeBinaryToWriter = function(message, writer) {
  var f = undefined;
  f = message.getType();
  if (f !== 0.0) {
    writer.writeEnum(
      1,
      f
    );
  }
  f = message.getEntryFunctionPayload();
  if (f != null) {
    writer.writeMessage(
      2,
      f,
      proto.aptos.transaction.testing1.v1.EntryFunctionPayload.serializeBinaryToWriter
    );
  }
};


/**
 * @enum {number}
 */
proto.aptos.transaction.testing1.v1.MultisigTransactionPayload.Type = {
  TYPE_UNSPECIFIED: 0,
  TYPE_ENTRY_FUNCTION_PAYLOAD: 1
};

/**
 * optional Type type = 1;
 * @return {!proto.aptos.transaction.testing1.v1.MultisigTransactionPayload.Type}
 */
proto.aptos.transaction.testing1.v1.MultisigTransactionPayload.prototype.getType = function() {
  return /** @type {!proto.aptos.transaction.testing1.v1.MultisigTransactionPayload.Type} */ (jspb.Message.getFieldWithDefault(this, 1, 0));
};


/**
 * @param {!proto.aptos.transaction.testing1.v1.MultisigTransactionPayload.Type} value
 * @return {!proto.aptos.transaction.testing1.v1.MultisigTransactionPayload} returns this
 */
proto.aptos.transaction.testing1.v1.MultisigTransactionPayload.prototype.setType = function(value) {
  return jspb.Message.setProto3EnumField(this, 1, value);
};


/**
 * optional EntryFunctionPayload entry_function_payload = 2;
 * @return {?proto.aptos.transaction.testing1.v1.EntryFunctionPayload}
 */
proto.aptos.transaction.testing1.v1.MultisigTransactionPayload.prototype.getEntryFunctionPayload = function() {
  return /** @type{?proto.aptos.transaction.testing1.v1.EntryFunctionPayload} */ (
    jspb.Message.getWrapperField(this, proto.aptos.transaction.testing1.v1.EntryFunctionPayload, 2));
};


/**
 * @param {?proto.aptos.transaction.testing1.v1.EntryFunctionPayload|undefined} value
 * @return {!proto.aptos.transaction.testing1.v1.MultisigTransactionPayload} returns this
*/
proto.aptos.transaction.testing1.v1.MultisigTransactionPayload.prototype.setEntryFunctionPayload = function(value) {
  return jspb.Message.setOneofWrapperField(this, 2, proto.aptos.transaction.testing1.v1.MultisigTransactionPayload.oneofGroups_[0], value);
};


/**
 * Clears the message field making it undefined.
 * @return {!proto.aptos.transaction.testing1.v1.MultisigTransactionPayload} returns this
 */
proto.aptos.transaction.testing1.v1.MultisigTransactionPayload.prototype.clearEntryFunctionPayload = function() {
  return this.setEntryFunctionPayload(undefined);
};


/**
 * Returns whether this field is set.
 * @return {boolean}
 */
proto.aptos.transaction.testing1.v1.MultisigTransactionPayload.prototype.hasEntryFunctionPayload = function() {
  return jspb.Message.getField(this, 2) != null;
};



/**
 * List of repeated fields within this message type.
 * @private {!Array<number>}
 * @const
 */
proto.aptos.transaction.testing1.v1.ModuleBundlePayload.repeatedFields_ = [1];



if (jspb.Message.GENERATE_TO_OBJECT) {
/**
 * Creates an object representation of this proto.
 * Field names that are reserved in JavaScript and will be renamed to pb_name.
 * Optional fields that are not set will be set to undefined.
 * To access a reserved field use, foo.pb_<name>, eg, foo.pb_default.
 * For the list of reserved names please see:
 *     net/proto2/compiler/js/internal/generator.cc#kKeyword.
 * @param {boolean=} opt_includeInstance Deprecated. whether to include the
 *     JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @return {!Object}
 */
proto.aptos.transaction.testing1.v1.ModuleBundlePayload.prototype.toObject = function(opt_includeInstance) {
  return proto.aptos.transaction.testing1.v1.ModuleBundlePayload.toObject(opt_includeInstance, this);
};


/**
 * Static version of the {@see toObject} method.
 * @param {boolean|undefined} includeInstance Deprecated. Whether to include
 *     the JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @param {!proto.aptos.transaction.testing1.v1.ModuleBundlePayload} msg The msg instance to transform.
 * @return {!Object}
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.aptos.transaction.testing1.v1.ModuleBundlePayload.toObject = function(includeInstance, msg) {
  var f, obj = {
    modulesList: jspb.Message.toObjectList(msg.getModulesList(),
    proto.aptos.transaction.testing1.v1.MoveModuleBytecode.toObject, includeInstance)
  };

  if (includeInstance) {
    obj.$jspbMessageInstance = msg;
  }
  return obj;
};
}


/**
 * Deserializes binary data (in protobuf wire format).
 * @param {jspb.ByteSource} bytes The bytes to deserialize.
 * @return {!proto.aptos.transaction.testing1.v1.ModuleBundlePayload}
 */
proto.aptos.transaction.testing1.v1.ModuleBundlePayload.deserializeBinary = function(bytes) {
  var reader = new jspb.BinaryReader(bytes);
  var msg = new proto.aptos.transaction.testing1.v1.ModuleBundlePayload;
  return proto.aptos.transaction.testing1.v1.ModuleBundlePayload.deserializeBinaryFromReader(msg, reader);
};


/**
 * Deserializes binary data (in protobuf wire format) from the
 * given reader into the given message object.
 * @param {!proto.aptos.transaction.testing1.v1.ModuleBundlePayload} msg The message object to deserialize into.
 * @param {!jspb.BinaryReader} reader The BinaryReader to use.
 * @return {!proto.aptos.transaction.testing1.v1.ModuleBundlePayload}
 */
proto.aptos.transaction.testing1.v1.ModuleBundlePayload.deserializeBinaryFromReader = function(msg, reader) {
  while (reader.nextField()) {
    if (reader.isEndGroup()) {
      break;
    }
    var field = reader.getFieldNumber();
    switch (field) {
    case 1:
      var value = new proto.aptos.transaction.testing1.v1.MoveModuleBytecode;
      reader.readMessage(value,proto.aptos.transaction.testing1.v1.MoveModuleBytecode.deserializeBinaryFromReader);
      msg.addModules(value);
      break;
    default:
      reader.skipField();
      break;
    }
  }
  return msg;
};


/**
 * Serializes the message to binary data (in protobuf wire format).
 * @return {!Uint8Array}
 */
proto.aptos.transaction.testing1.v1.ModuleBundlePayload.prototype.serializeBinary = function() {
  var writer = new jspb.BinaryWriter();
  proto.aptos.transaction.testing1.v1.ModuleBundlePayload.serializeBinaryToWriter(this, writer);
  return writer.getResultBuffer();
};


/**
 * Serializes the given message to binary data (in protobuf wire
 * format), writing to the given BinaryWriter.
 * @param {!proto.aptos.transaction.testing1.v1.ModuleBundlePayload} message
 * @param {!jspb.BinaryWriter} writer
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.aptos.transaction.testing1.v1.ModuleBundlePayload.serializeBinaryToWriter = function(message, writer) {
  var f = undefined;
  f = message.getModulesList();
  if (f.length > 0) {
    writer.writeRepeatedMessage(
      1,
      f,
      proto.aptos.transaction.testing1.v1.MoveModuleBytecode.serializeBinaryToWriter
    );
  }
};


/**
 * repeated MoveModuleBytecode modules = 1;
 * @return {!Array<!proto.aptos.transaction.testing1.v1.MoveModuleBytecode>}
 */
proto.aptos.transaction.testing1.v1.ModuleBundlePayload.prototype.getModulesList = function() {
  return /** @type{!Array<!proto.aptos.transaction.testing1.v1.MoveModuleBytecode>} */ (
    jspb.Message.getRepeatedWrapperField(this, proto.aptos.transaction.testing1.v1.MoveModuleBytecode, 1));
};


/**
 * @param {!Array<!proto.aptos.transaction.testing1.v1.MoveModuleBytecode>} value
 * @return {!proto.aptos.transaction.testing1.v1.ModuleBundlePayload} returns this
*/
proto.aptos.transaction.testing1.v1.ModuleBundlePayload.prototype.setModulesList = function(value) {
  return jspb.Message.setRepeatedWrapperField(this, 1, value);
};


/**
 * @param {!proto.aptos.transaction.testing1.v1.MoveModuleBytecode=} opt_value
 * @param {number=} opt_index
 * @return {!proto.aptos.transaction.testing1.v1.MoveModuleBytecode}
 */
proto.aptos.transaction.testing1.v1.ModuleBundlePayload.prototype.addModules = function(opt_value, opt_index) {
  return jspb.Message.addToRepeatedWrapperField(this, 1, opt_value, proto.aptos.transaction.testing1.v1.MoveModuleBytecode, opt_index);
};


/**
 * Clears the list making it empty but non-null.
 * @return {!proto.aptos.transaction.testing1.v1.ModuleBundlePayload} returns this
 */
proto.aptos.transaction.testing1.v1.ModuleBundlePayload.prototype.clearModulesList = function() {
  return this.setModulesList([]);
};





if (jspb.Message.GENERATE_TO_OBJECT) {
/**
 * Creates an object representation of this proto.
 * Field names that are reserved in JavaScript and will be renamed to pb_name.
 * Optional fields that are not set will be set to undefined.
 * To access a reserved field use, foo.pb_<name>, eg, foo.pb_default.
 * For the list of reserved names please see:
 *     net/proto2/compiler/js/internal/generator.cc#kKeyword.
 * @param {boolean=} opt_includeInstance Deprecated. whether to include the
 *     JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @return {!Object}
 */
proto.aptos.transaction.testing1.v1.MoveModuleBytecode.prototype.toObject = function(opt_includeInstance) {
  return proto.aptos.transaction.testing1.v1.MoveModuleBytecode.toObject(opt_includeInstance, this);
};


/**
 * Static version of the {@see toObject} method.
 * @param {boolean|undefined} includeInstance Deprecated. Whether to include
 *     the JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @param {!proto.aptos.transaction.testing1.v1.MoveModuleBytecode} msg The msg instance to transform.
 * @return {!Object}
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.aptos.transaction.testing1.v1.MoveModuleBytecode.toObject = function(includeInstance, msg) {
  var f, obj = {
    bytecode: msg.getBytecode_asB64(),
    abi: (f = msg.getAbi()) && proto.aptos.transaction.testing1.v1.MoveModule.toObject(includeInstance, f)
  };

  if (includeInstance) {
    obj.$jspbMessageInstance = msg;
  }
  return obj;
};
}


/**
 * Deserializes binary data (in protobuf wire format).
 * @param {jspb.ByteSource} bytes The bytes to deserialize.
 * @return {!proto.aptos.transaction.testing1.v1.MoveModuleBytecode}
 */
proto.aptos.transaction.testing1.v1.MoveModuleBytecode.deserializeBinary = function(bytes) {
  var reader = new jspb.BinaryReader(bytes);
  var msg = new proto.aptos.transaction.testing1.v1.MoveModuleBytecode;
  return proto.aptos.transaction.testing1.v1.MoveModuleBytecode.deserializeBinaryFromReader(msg, reader);
};


/**
 * Deserializes binary data (in protobuf wire format) from the
 * given reader into the given message object.
 * @param {!proto.aptos.transaction.testing1.v1.MoveModuleBytecode} msg The message object to deserialize into.
 * @param {!jspb.BinaryReader} reader The BinaryReader to use.
 * @return {!proto.aptos.transaction.testing1.v1.MoveModuleBytecode}
 */
proto.aptos.transaction.testing1.v1.MoveModuleBytecode.deserializeBinaryFromReader = function(msg, reader) {
  while (reader.nextField()) {
    if (reader.isEndGroup()) {
      break;
    }
    var field = reader.getFieldNumber();
    switch (field) {
    case 1:
      var value = /** @type {!Uint8Array} */ (reader.readBytes());
      msg.setBytecode(value);
      break;
    case 2:
      var value = new proto.aptos.transaction.testing1.v1.MoveModule;
      reader.readMessage(value,proto.aptos.transaction.testing1.v1.MoveModule.deserializeBinaryFromReader);
      msg.setAbi(value);
      break;
    default:
      reader.skipField();
      break;
    }
  }
  return msg;
};


/**
 * Serializes the message to binary data (in protobuf wire format).
 * @return {!Uint8Array}
 */
proto.aptos.transaction.testing1.v1.MoveModuleBytecode.prototype.serializeBinary = function() {
  var writer = new jspb.BinaryWriter();
  proto.aptos.transaction.testing1.v1.MoveModuleBytecode.serializeBinaryToWriter(this, writer);
  return writer.getResultBuffer();
};


/**
 * Serializes the given message to binary data (in protobuf wire
 * format), writing to the given BinaryWriter.
 * @param {!proto.aptos.transaction.testing1.v1.MoveModuleBytecode} message
 * @param {!jspb.BinaryWriter} writer
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.aptos.transaction.testing1.v1.MoveModuleBytecode.serializeBinaryToWriter = function(message, writer) {
  var f = undefined;
  f = message.getBytecode_asU8();
  if (f.length > 0) {
    writer.writeBytes(
      1,
      f
    );
  }
  f = message.getAbi();
  if (f != null) {
    writer.writeMessage(
      2,
      f,
      proto.aptos.transaction.testing1.v1.MoveModule.serializeBinaryToWriter
    );
  }
};


/**
 * optional bytes bytecode = 1;
 * @return {!(string|Uint8Array)}
 */
proto.aptos.transaction.testing1.v1.MoveModuleBytecode.prototype.getBytecode = function() {
  return /** @type {!(string|Uint8Array)} */ (jspb.Message.getFieldWithDefault(this, 1, ""));
};


/**
 * optional bytes bytecode = 1;
 * This is a type-conversion wrapper around `getBytecode()`
 * @return {string}
 */
proto.aptos.transaction.testing1.v1.MoveModuleBytecode.prototype.getBytecode_asB64 = function() {
  return /** @type {string} */ (jspb.Message.bytesAsB64(
      this.getBytecode()));
};


/**
 * optional bytes bytecode = 1;
 * Note that Uint8Array is not supported on all browsers.
 * @see http://caniuse.com/Uint8Array
 * This is a type-conversion wrapper around `getBytecode()`
 * @return {!Uint8Array}
 */
proto.aptos.transaction.testing1.v1.MoveModuleBytecode.prototype.getBytecode_asU8 = function() {
  return /** @type {!Uint8Array} */ (jspb.Message.bytesAsU8(
      this.getBytecode()));
};


/**
 * @param {!(string|Uint8Array)} value
 * @return {!proto.aptos.transaction.testing1.v1.MoveModuleBytecode} returns this
 */
proto.aptos.transaction.testing1.v1.MoveModuleBytecode.prototype.setBytecode = function(value) {
  return jspb.Message.setProto3BytesField(this, 1, value);
};


/**
 * optional MoveModule abi = 2;
 * @return {?proto.aptos.transaction.testing1.v1.MoveModule}
 */
proto.aptos.transaction.testing1.v1.MoveModuleBytecode.prototype.getAbi = function() {
  return /** @type{?proto.aptos.transaction.testing1.v1.MoveModule} */ (
    jspb.Message.getWrapperField(this, proto.aptos.transaction.testing1.v1.MoveModule, 2));
};


/**
 * @param {?proto.aptos.transaction.testing1.v1.MoveModule|undefined} value
 * @return {!proto.aptos.transaction.testing1.v1.MoveModuleBytecode} returns this
*/
proto.aptos.transaction.testing1.v1.MoveModuleBytecode.prototype.setAbi = function(value) {
  return jspb.Message.setWrapperField(this, 2, value);
};


/**
 * Clears the message field making it undefined.
 * @return {!proto.aptos.transaction.testing1.v1.MoveModuleBytecode} returns this
 */
proto.aptos.transaction.testing1.v1.MoveModuleBytecode.prototype.clearAbi = function() {
  return this.setAbi(undefined);
};


/**
 * Returns whether this field is set.
 * @return {boolean}
 */
proto.aptos.transaction.testing1.v1.MoveModuleBytecode.prototype.hasAbi = function() {
  return jspb.Message.getField(this, 2) != null;
};



/**
 * List of repeated fields within this message type.
 * @private {!Array<number>}
 * @const
 */
proto.aptos.transaction.testing1.v1.MoveModule.repeatedFields_ = [3,4,5];



if (jspb.Message.GENERATE_TO_OBJECT) {
/**
 * Creates an object representation of this proto.
 * Field names that are reserved in JavaScript and will be renamed to pb_name.
 * Optional fields that are not set will be set to undefined.
 * To access a reserved field use, foo.pb_<name>, eg, foo.pb_default.
 * For the list of reserved names please see:
 *     net/proto2/compiler/js/internal/generator.cc#kKeyword.
 * @param {boolean=} opt_includeInstance Deprecated. whether to include the
 *     JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @return {!Object}
 */
proto.aptos.transaction.testing1.v1.MoveModule.prototype.toObject = function(opt_includeInstance) {
  return proto.aptos.transaction.testing1.v1.MoveModule.toObject(opt_includeInstance, this);
};


/**
 * Static version of the {@see toObject} method.
 * @param {boolean|undefined} includeInstance Deprecated. Whether to include
 *     the JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @param {!proto.aptos.transaction.testing1.v1.MoveModule} msg The msg instance to transform.
 * @return {!Object}
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.aptos.transaction.testing1.v1.MoveModule.toObject = function(includeInstance, msg) {
  var f, obj = {
    address: jspb.Message.getFieldWithDefault(msg, 1, ""),
    name: jspb.Message.getFieldWithDefault(msg, 2, ""),
    friendsList: jspb.Message.toObjectList(msg.getFriendsList(),
    proto.aptos.transaction.testing1.v1.MoveModuleId.toObject, includeInstance),
    exposedFunctionsList: jspb.Message.toObjectList(msg.getExposedFunctionsList(),
    proto.aptos.transaction.testing1.v1.MoveFunction.toObject, includeInstance),
    structsList: jspb.Message.toObjectList(msg.getStructsList(),
    proto.aptos.transaction.testing1.v1.MoveStruct.toObject, includeInstance)
  };

  if (includeInstance) {
    obj.$jspbMessageInstance = msg;
  }
  return obj;
};
}


/**
 * Deserializes binary data (in protobuf wire format).
 * @param {jspb.ByteSource} bytes The bytes to deserialize.
 * @return {!proto.aptos.transaction.testing1.v1.MoveModule}
 */
proto.aptos.transaction.testing1.v1.MoveModule.deserializeBinary = function(bytes) {
  var reader = new jspb.BinaryReader(bytes);
  var msg = new proto.aptos.transaction.testing1.v1.MoveModule;
  return proto.aptos.transaction.testing1.v1.MoveModule.deserializeBinaryFromReader(msg, reader);
};


/**
 * Deserializes binary data (in protobuf wire format) from the
 * given reader into the given message object.
 * @param {!proto.aptos.transaction.testing1.v1.MoveModule} msg The message object to deserialize into.
 * @param {!jspb.BinaryReader} reader The BinaryReader to use.
 * @return {!proto.aptos.transaction.testing1.v1.MoveModule}
 */
proto.aptos.transaction.testing1.v1.MoveModule.deserializeBinaryFromReader = function(msg, reader) {
  while (reader.nextField()) {
    if (reader.isEndGroup()) {
      break;
    }
    var field = reader.getFieldNumber();
    switch (field) {
    case 1:
      var value = /** @type {string} */ (reader.readString());
      msg.setAddress(value);
      break;
    case 2:
      var value = /** @type {string} */ (reader.readString());
      msg.setName(value);
      break;
    case 3:
      var value = new proto.aptos.transaction.testing1.v1.MoveModuleId;
      reader.readMessage(value,proto.aptos.transaction.testing1.v1.MoveModuleId.deserializeBinaryFromReader);
      msg.addFriends(value);
      break;
    case 4:
      var value = new proto.aptos.transaction.testing1.v1.MoveFunction;
      reader.readMessage(value,proto.aptos.transaction.testing1.v1.MoveFunction.deserializeBinaryFromReader);
      msg.addExposedFunctions(value);
      break;
    case 5:
      var value = new proto.aptos.transaction.testing1.v1.MoveStruct;
      reader.readMessage(value,proto.aptos.transaction.testing1.v1.MoveStruct.deserializeBinaryFromReader);
      msg.addStructs(value);
      break;
    default:
      reader.skipField();
      break;
    }
  }
  return msg;
};


/**
 * Serializes the message to binary data (in protobuf wire format).
 * @return {!Uint8Array}
 */
proto.aptos.transaction.testing1.v1.MoveModule.prototype.serializeBinary = function() {
  var writer = new jspb.BinaryWriter();
  proto.aptos.transaction.testing1.v1.MoveModule.serializeBinaryToWriter(this, writer);
  return writer.getResultBuffer();
};


/**
 * Serializes the given message to binary data (in protobuf wire
 * format), writing to the given BinaryWriter.
 * @param {!proto.aptos.transaction.testing1.v1.MoveModule} message
 * @param {!jspb.BinaryWriter} writer
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.aptos.transaction.testing1.v1.MoveModule.serializeBinaryToWriter = function(message, writer) {
  var f = undefined;
  f = message.getAddress();
  if (f.length > 0) {
    writer.writeString(
      1,
      f
    );
  }
  f = message.getName();
  if (f.length > 0) {
    writer.writeString(
      2,
      f
    );
  }
  f = message.getFriendsList();
  if (f.length > 0) {
    writer.writeRepeatedMessage(
      3,
      f,
      proto.aptos.transaction.testing1.v1.MoveModuleId.serializeBinaryToWriter
    );
  }
  f = message.getExposedFunctionsList();
  if (f.length > 0) {
    writer.writeRepeatedMessage(
      4,
      f,
      proto.aptos.transaction.testing1.v1.MoveFunction.serializeBinaryToWriter
    );
  }
  f = message.getStructsList();
  if (f.length > 0) {
    writer.writeRepeatedMessage(
      5,
      f,
      proto.aptos.transaction.testing1.v1.MoveStruct.serializeBinaryToWriter
    );
  }
};


/**
 * optional string address = 1;
 * @return {string}
 */
proto.aptos.transaction.testing1.v1.MoveModule.prototype.getAddress = function() {
  return /** @type {string} */ (jspb.Message.getFieldWithDefault(this, 1, ""));
};


/**
 * @param {string} value
 * @return {!proto.aptos.transaction.testing1.v1.MoveModule} returns this
 */
proto.aptos.transaction.testing1.v1.MoveModule.prototype.setAddress = function(value) {
  return jspb.Message.setProto3StringField(this, 1, value);
};


/**
 * optional string name = 2;
 * @return {string}
 */
proto.aptos.transaction.testing1.v1.MoveModule.prototype.getName = function() {
  return /** @type {string} */ (jspb.Message.getFieldWithDefault(this, 2, ""));
};


/**
 * @param {string} value
 * @return {!proto.aptos.transaction.testing1.v1.MoveModule} returns this
 */
proto.aptos.transaction.testing1.v1.MoveModule.prototype.setName = function(value) {
  return jspb.Message.setProto3StringField(this, 2, value);
};


/**
 * repeated MoveModuleId friends = 3;
 * @return {!Array<!proto.aptos.transaction.testing1.v1.MoveModuleId>}
 */
proto.aptos.transaction.testing1.v1.MoveModule.prototype.getFriendsList = function() {
  return /** @type{!Array<!proto.aptos.transaction.testing1.v1.MoveModuleId>} */ (
    jspb.Message.getRepeatedWrapperField(this, proto.aptos.transaction.testing1.v1.MoveModuleId, 3));
};


/**
 * @param {!Array<!proto.aptos.transaction.testing1.v1.MoveModuleId>} value
 * @return {!proto.aptos.transaction.testing1.v1.MoveModule} returns this
*/
proto.aptos.transaction.testing1.v1.MoveModule.prototype.setFriendsList = function(value) {
  return jspb.Message.setRepeatedWrapperField(this, 3, value);
};


/**
 * @param {!proto.aptos.transaction.testing1.v1.MoveModuleId=} opt_value
 * @param {number=} opt_index
 * @return {!proto.aptos.transaction.testing1.v1.MoveModuleId}
 */
proto.aptos.transaction.testing1.v1.MoveModule.prototype.addFriends = function(opt_value, opt_index) {
  return jspb.Message.addToRepeatedWrapperField(this, 3, opt_value, proto.aptos.transaction.testing1.v1.MoveModuleId, opt_index);
};


/**
 * Clears the list making it empty but non-null.
 * @return {!proto.aptos.transaction.testing1.v1.MoveModule} returns this
 */
proto.aptos.transaction.testing1.v1.MoveModule.prototype.clearFriendsList = function() {
  return this.setFriendsList([]);
};


/**
 * repeated MoveFunction exposed_functions = 4;
 * @return {!Array<!proto.aptos.transaction.testing1.v1.MoveFunction>}
 */
proto.aptos.transaction.testing1.v1.MoveModule.prototype.getExposedFunctionsList = function() {
  return /** @type{!Array<!proto.aptos.transaction.testing1.v1.MoveFunction>} */ (
    jspb.Message.getRepeatedWrapperField(this, proto.aptos.transaction.testing1.v1.MoveFunction, 4));
};


/**
 * @param {!Array<!proto.aptos.transaction.testing1.v1.MoveFunction>} value
 * @return {!proto.aptos.transaction.testing1.v1.MoveModule} returns this
*/
proto.aptos.transaction.testing1.v1.MoveModule.prototype.setExposedFunctionsList = function(value) {
  return jspb.Message.setRepeatedWrapperField(this, 4, value);
};


/**
 * @param {!proto.aptos.transaction.testing1.v1.MoveFunction=} opt_value
 * @param {number=} opt_index
 * @return {!proto.aptos.transaction.testing1.v1.MoveFunction}
 */
proto.aptos.transaction.testing1.v1.MoveModule.prototype.addExposedFunctions = function(opt_value, opt_index) {
  return jspb.Message.addToRepeatedWrapperField(this, 4, opt_value, proto.aptos.transaction.testing1.v1.MoveFunction, opt_index);
};


/**
 * Clears the list making it empty but non-null.
 * @return {!proto.aptos.transaction.testing1.v1.MoveModule} returns this
 */
proto.aptos.transaction.testing1.v1.MoveModule.prototype.clearExposedFunctionsList = function() {
  return this.setExposedFunctionsList([]);
};


/**
 * repeated MoveStruct structs = 5;
 * @return {!Array<!proto.aptos.transaction.testing1.v1.MoveStruct>}
 */
proto.aptos.transaction.testing1.v1.MoveModule.prototype.getStructsList = function() {
  return /** @type{!Array<!proto.aptos.transaction.testing1.v1.MoveStruct>} */ (
    jspb.Message.getRepeatedWrapperField(this, proto.aptos.transaction.testing1.v1.MoveStruct, 5));
};


/**
 * @param {!Array<!proto.aptos.transaction.testing1.v1.MoveStruct>} value
 * @return {!proto.aptos.transaction.testing1.v1.MoveModule} returns this
*/
proto.aptos.transaction.testing1.v1.MoveModule.prototype.setStructsList = function(value) {
  return jspb.Message.setRepeatedWrapperField(this, 5, value);
};


/**
 * @param {!proto.aptos.transaction.testing1.v1.MoveStruct=} opt_value
 * @param {number=} opt_index
 * @return {!proto.aptos.transaction.testing1.v1.MoveStruct}
 */
proto.aptos.transaction.testing1.v1.MoveModule.prototype.addStructs = function(opt_value, opt_index) {
  return jspb.Message.addToRepeatedWrapperField(this, 5, opt_value, proto.aptos.transaction.testing1.v1.MoveStruct, opt_index);
};


/**
 * Clears the list making it empty but non-null.
 * @return {!proto.aptos.transaction.testing1.v1.MoveModule} returns this
 */
proto.aptos.transaction.testing1.v1.MoveModule.prototype.clearStructsList = function() {
  return this.setStructsList([]);
};



/**
 * List of repeated fields within this message type.
 * @private {!Array<number>}
 * @const
 */
proto.aptos.transaction.testing1.v1.MoveFunction.repeatedFields_ = [4,5,6];



if (jspb.Message.GENERATE_TO_OBJECT) {
/**
 * Creates an object representation of this proto.
 * Field names that are reserved in JavaScript and will be renamed to pb_name.
 * Optional fields that are not set will be set to undefined.
 * To access a reserved field use, foo.pb_<name>, eg, foo.pb_default.
 * For the list of reserved names please see:
 *     net/proto2/compiler/js/internal/generator.cc#kKeyword.
 * @param {boolean=} opt_includeInstance Deprecated. whether to include the
 *     JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @return {!Object}
 */
proto.aptos.transaction.testing1.v1.MoveFunction.prototype.toObject = function(opt_includeInstance) {
  return proto.aptos.transaction.testing1.v1.MoveFunction.toObject(opt_includeInstance, this);
};


/**
 * Static version of the {@see toObject} method.
 * @param {boolean|undefined} includeInstance Deprecated. Whether to include
 *     the JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @param {!proto.aptos.transaction.testing1.v1.MoveFunction} msg The msg instance to transform.
 * @return {!Object}
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.aptos.transaction.testing1.v1.MoveFunction.toObject = function(includeInstance, msg) {
  var f, obj = {
    name: jspb.Message.getFieldWithDefault(msg, 1, ""),
    visibility: jspb.Message.getFieldWithDefault(msg, 2, 0),
    isEntry: jspb.Message.getBooleanFieldWithDefault(msg, 3, false),
    genericTypeParamsList: jspb.Message.toObjectList(msg.getGenericTypeParamsList(),
    proto.aptos.transaction.testing1.v1.MoveFunctionGenericTypeParam.toObject, includeInstance),
    paramsList: jspb.Message.toObjectList(msg.getParamsList(),
    proto.aptos.transaction.testing1.v1.MoveType.toObject, includeInstance),
    returnList: jspb.Message.toObjectList(msg.getReturnList(),
    proto.aptos.transaction.testing1.v1.MoveType.toObject, includeInstance)
  };

  if (includeInstance) {
    obj.$jspbMessageInstance = msg;
  }
  return obj;
};
}


/**
 * Deserializes binary data (in protobuf wire format).
 * @param {jspb.ByteSource} bytes The bytes to deserialize.
 * @return {!proto.aptos.transaction.testing1.v1.MoveFunction}
 */
proto.aptos.transaction.testing1.v1.MoveFunction.deserializeBinary = function(bytes) {
  var reader = new jspb.BinaryReader(bytes);
  var msg = new proto.aptos.transaction.testing1.v1.MoveFunction;
  return proto.aptos.transaction.testing1.v1.MoveFunction.deserializeBinaryFromReader(msg, reader);
};


/**
 * Deserializes binary data (in protobuf wire format) from the
 * given reader into the given message object.
 * @param {!proto.aptos.transaction.testing1.v1.MoveFunction} msg The message object to deserialize into.
 * @param {!jspb.BinaryReader} reader The BinaryReader to use.
 * @return {!proto.aptos.transaction.testing1.v1.MoveFunction}
 */
proto.aptos.transaction.testing1.v1.MoveFunction.deserializeBinaryFromReader = function(msg, reader) {
  while (reader.nextField()) {
    if (reader.isEndGroup()) {
      break;
    }
    var field = reader.getFieldNumber();
    switch (field) {
    case 1:
      var value = /** @type {string} */ (reader.readString());
      msg.setName(value);
      break;
    case 2:
      var value = /** @type {!proto.aptos.transaction.testing1.v1.MoveFunction.Visibility} */ (reader.readEnum());
      msg.setVisibility(value);
      break;
    case 3:
      var value = /** @type {boolean} */ (reader.readBool());
      msg.setIsEntry(value);
      break;
    case 4:
      var value = new proto.aptos.transaction.testing1.v1.MoveFunctionGenericTypeParam;
      reader.readMessage(value,proto.aptos.transaction.testing1.v1.MoveFunctionGenericTypeParam.deserializeBinaryFromReader);
      msg.addGenericTypeParams(value);
      break;
    case 5:
      var value = new proto.aptos.transaction.testing1.v1.MoveType;
      reader.readMessage(value,proto.aptos.transaction.testing1.v1.MoveType.deserializeBinaryFromReader);
      msg.addParams(value);
      break;
    case 6:
      var value = new proto.aptos.transaction.testing1.v1.MoveType;
      reader.readMessage(value,proto.aptos.transaction.testing1.v1.MoveType.deserializeBinaryFromReader);
      msg.addReturn(value);
      break;
    default:
      reader.skipField();
      break;
    }
  }
  return msg;
};


/**
 * Serializes the message to binary data (in protobuf wire format).
 * @return {!Uint8Array}
 */
proto.aptos.transaction.testing1.v1.MoveFunction.prototype.serializeBinary = function() {
  var writer = new jspb.BinaryWriter();
  proto.aptos.transaction.testing1.v1.MoveFunction.serializeBinaryToWriter(this, writer);
  return writer.getResultBuffer();
};


/**
 * Serializes the given message to binary data (in protobuf wire
 * format), writing to the given BinaryWriter.
 * @param {!proto.aptos.transaction.testing1.v1.MoveFunction} message
 * @param {!jspb.BinaryWriter} writer
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.aptos.transaction.testing1.v1.MoveFunction.serializeBinaryToWriter = function(message, writer) {
  var f = undefined;
  f = message.getName();
  if (f.length > 0) {
    writer.writeString(
      1,
      f
    );
  }
  f = message.getVisibility();
  if (f !== 0.0) {
    writer.writeEnum(
      2,
      f
    );
  }
  f = message.getIsEntry();
  if (f) {
    writer.writeBool(
      3,
      f
    );
  }
  f = message.getGenericTypeParamsList();
  if (f.length > 0) {
    writer.writeRepeatedMessage(
      4,
      f,
      proto.aptos.transaction.testing1.v1.MoveFunctionGenericTypeParam.serializeBinaryToWriter
    );
  }
  f = message.getParamsList();
  if (f.length > 0) {
    writer.writeRepeatedMessage(
      5,
      f,
      proto.aptos.transaction.testing1.v1.MoveType.serializeBinaryToWriter
    );
  }
  f = message.getReturnList();
  if (f.length > 0) {
    writer.writeRepeatedMessage(
      6,
      f,
      proto.aptos.transaction.testing1.v1.MoveType.serializeBinaryToWriter
    );
  }
};


/**
 * @enum {number}
 */
proto.aptos.transaction.testing1.v1.MoveFunction.Visibility = {
  VISIBILITY_UNSPECIFIED: 0,
  VISIBILITY_PRIVATE: 1,
  VISIBILITY_PUBLIC: 2,
  VISIBILITY_FRIEND: 3
};

/**
 * optional string name = 1;
 * @return {string}
 */
proto.aptos.transaction.testing1.v1.MoveFunction.prototype.getName = function() {
  return /** @type {string} */ (jspb.Message.getFieldWithDefault(this, 1, ""));
};


/**
 * @param {string} value
 * @return {!proto.aptos.transaction.testing1.v1.MoveFunction} returns this
 */
proto.aptos.transaction.testing1.v1.MoveFunction.prototype.setName = function(value) {
  return jspb.Message.setProto3StringField(this, 1, value);
};


/**
 * optional Visibility visibility = 2;
 * @return {!proto.aptos.transaction.testing1.v1.MoveFunction.Visibility}
 */
proto.aptos.transaction.testing1.v1.MoveFunction.prototype.getVisibility = function() {
  return /** @type {!proto.aptos.transaction.testing1.v1.MoveFunction.Visibility} */ (jspb.Message.getFieldWithDefault(this, 2, 0));
};


/**
 * @param {!proto.aptos.transaction.testing1.v1.MoveFunction.Visibility} value
 * @return {!proto.aptos.transaction.testing1.v1.MoveFunction} returns this
 */
proto.aptos.transaction.testing1.v1.MoveFunction.prototype.setVisibility = function(value) {
  return jspb.Message.setProto3EnumField(this, 2, value);
};


/**
 * optional bool is_entry = 3;
 * @return {boolean}
 */
proto.aptos.transaction.testing1.v1.MoveFunction.prototype.getIsEntry = function() {
  return /** @type {boolean} */ (jspb.Message.getBooleanFieldWithDefault(this, 3, false));
};


/**
 * @param {boolean} value
 * @return {!proto.aptos.transaction.testing1.v1.MoveFunction} returns this
 */
proto.aptos.transaction.testing1.v1.MoveFunction.prototype.setIsEntry = function(value) {
  return jspb.Message.setProto3BooleanField(this, 3, value);
};


/**
 * repeated MoveFunctionGenericTypeParam generic_type_params = 4;
 * @return {!Array<!proto.aptos.transaction.testing1.v1.MoveFunctionGenericTypeParam>}
 */
proto.aptos.transaction.testing1.v1.MoveFunction.prototype.getGenericTypeParamsList = function() {
  return /** @type{!Array<!proto.aptos.transaction.testing1.v1.MoveFunctionGenericTypeParam>} */ (
    jspb.Message.getRepeatedWrapperField(this, proto.aptos.transaction.testing1.v1.MoveFunctionGenericTypeParam, 4));
};


/**
 * @param {!Array<!proto.aptos.transaction.testing1.v1.MoveFunctionGenericTypeParam>} value
 * @return {!proto.aptos.transaction.testing1.v1.MoveFunction} returns this
*/
proto.aptos.transaction.testing1.v1.MoveFunction.prototype.setGenericTypeParamsList = function(value) {
  return jspb.Message.setRepeatedWrapperField(this, 4, value);
};


/**
 * @param {!proto.aptos.transaction.testing1.v1.MoveFunctionGenericTypeParam=} opt_value
 * @param {number=} opt_index
 * @return {!proto.aptos.transaction.testing1.v1.MoveFunctionGenericTypeParam}
 */
proto.aptos.transaction.testing1.v1.MoveFunction.prototype.addGenericTypeParams = function(opt_value, opt_index) {
  return jspb.Message.addToRepeatedWrapperField(this, 4, opt_value, proto.aptos.transaction.testing1.v1.MoveFunctionGenericTypeParam, opt_index);
};


/**
 * Clears the list making it empty but non-null.
 * @return {!proto.aptos.transaction.testing1.v1.MoveFunction} returns this
 */
proto.aptos.transaction.testing1.v1.MoveFunction.prototype.clearGenericTypeParamsList = function() {
  return this.setGenericTypeParamsList([]);
};


/**
 * repeated MoveType params = 5;
 * @return {!Array<!proto.aptos.transaction.testing1.v1.MoveType>}
 */
proto.aptos.transaction.testing1.v1.MoveFunction.prototype.getParamsList = function() {
  return /** @type{!Array<!proto.aptos.transaction.testing1.v1.MoveType>} */ (
    jspb.Message.getRepeatedWrapperField(this, proto.aptos.transaction.testing1.v1.MoveType, 5));
};


/**
 * @param {!Array<!proto.aptos.transaction.testing1.v1.MoveType>} value
 * @return {!proto.aptos.transaction.testing1.v1.MoveFunction} returns this
*/
proto.aptos.transaction.testing1.v1.MoveFunction.prototype.setParamsList = function(value) {
  return jspb.Message.setRepeatedWrapperField(this, 5, value);
};


/**
 * @param {!proto.aptos.transaction.testing1.v1.MoveType=} opt_value
 * @param {number=} opt_index
 * @return {!proto.aptos.transaction.testing1.v1.MoveType}
 */
proto.aptos.transaction.testing1.v1.MoveFunction.prototype.addParams = function(opt_value, opt_index) {
  return jspb.Message.addToRepeatedWrapperField(this, 5, opt_value, proto.aptos.transaction.testing1.v1.MoveType, opt_index);
};


/**
 * Clears the list making it empty but non-null.
 * @return {!proto.aptos.transaction.testing1.v1.MoveFunction} returns this
 */
proto.aptos.transaction.testing1.v1.MoveFunction.prototype.clearParamsList = function() {
  return this.setParamsList([]);
};


/**
 * repeated MoveType return = 6;
 * @return {!Array<!proto.aptos.transaction.testing1.v1.MoveType>}
 */
proto.aptos.transaction.testing1.v1.MoveFunction.prototype.getReturnList = function() {
  return /** @type{!Array<!proto.aptos.transaction.testing1.v1.MoveType>} */ (
    jspb.Message.getRepeatedWrapperField(this, proto.aptos.transaction.testing1.v1.MoveType, 6));
};


/**
 * @param {!Array<!proto.aptos.transaction.testing1.v1.MoveType>} value
 * @return {!proto.aptos.transaction.testing1.v1.MoveFunction} returns this
*/
proto.aptos.transaction.testing1.v1.MoveFunction.prototype.setReturnList = function(value) {
  return jspb.Message.setRepeatedWrapperField(this, 6, value);
};


/**
 * @param {!proto.aptos.transaction.testing1.v1.MoveType=} opt_value
 * @param {number=} opt_index
 * @return {!proto.aptos.transaction.testing1.v1.MoveType}
 */
proto.aptos.transaction.testing1.v1.MoveFunction.prototype.addReturn = function(opt_value, opt_index) {
  return jspb.Message.addToRepeatedWrapperField(this, 6, opt_value, proto.aptos.transaction.testing1.v1.MoveType, opt_index);
};


/**
 * Clears the list making it empty but non-null.
 * @return {!proto.aptos.transaction.testing1.v1.MoveFunction} returns this
 */
proto.aptos.transaction.testing1.v1.MoveFunction.prototype.clearReturnList = function() {
  return this.setReturnList([]);
};



/**
 * List of repeated fields within this message type.
 * @private {!Array<number>}
 * @const
 */
proto.aptos.transaction.testing1.v1.MoveStruct.repeatedFields_ = [3,4,5];



if (jspb.Message.GENERATE_TO_OBJECT) {
/**
 * Creates an object representation of this proto.
 * Field names that are reserved in JavaScript and will be renamed to pb_name.
 * Optional fields that are not set will be set to undefined.
 * To access a reserved field use, foo.pb_<name>, eg, foo.pb_default.
 * For the list of reserved names please see:
 *     net/proto2/compiler/js/internal/generator.cc#kKeyword.
 * @param {boolean=} opt_includeInstance Deprecated. whether to include the
 *     JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @return {!Object}
 */
proto.aptos.transaction.testing1.v1.MoveStruct.prototype.toObject = function(opt_includeInstance) {
  return proto.aptos.transaction.testing1.v1.MoveStruct.toObject(opt_includeInstance, this);
};


/**
 * Static version of the {@see toObject} method.
 * @param {boolean|undefined} includeInstance Deprecated. Whether to include
 *     the JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @param {!proto.aptos.transaction.testing1.v1.MoveStruct} msg The msg instance to transform.
 * @return {!Object}
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.aptos.transaction.testing1.v1.MoveStruct.toObject = function(includeInstance, msg) {
  var f, obj = {
    name: jspb.Message.getFieldWithDefault(msg, 1, ""),
    isNative: jspb.Message.getBooleanFieldWithDefault(msg, 2, false),
    abilitiesList: (f = jspb.Message.getRepeatedField(msg, 3)) == null ? undefined : f,
    genericTypeParamsList: jspb.Message.toObjectList(msg.getGenericTypeParamsList(),
    proto.aptos.transaction.testing1.v1.MoveStructGenericTypeParam.toObject, includeInstance),
    fieldsList: jspb.Message.toObjectList(msg.getFieldsList(),
    proto.aptos.transaction.testing1.v1.MoveStructField.toObject, includeInstance)
  };

  if (includeInstance) {
    obj.$jspbMessageInstance = msg;
  }
  return obj;
};
}


/**
 * Deserializes binary data (in protobuf wire format).
 * @param {jspb.ByteSource} bytes The bytes to deserialize.
 * @return {!proto.aptos.transaction.testing1.v1.MoveStruct}
 */
proto.aptos.transaction.testing1.v1.MoveStruct.deserializeBinary = function(bytes) {
  var reader = new jspb.BinaryReader(bytes);
  var msg = new proto.aptos.transaction.testing1.v1.MoveStruct;
  return proto.aptos.transaction.testing1.v1.MoveStruct.deserializeBinaryFromReader(msg, reader);
};


/**
 * Deserializes binary data (in protobuf wire format) from the
 * given reader into the given message object.
 * @param {!proto.aptos.transaction.testing1.v1.MoveStruct} msg The message object to deserialize into.
 * @param {!jspb.BinaryReader} reader The BinaryReader to use.
 * @return {!proto.aptos.transaction.testing1.v1.MoveStruct}
 */
proto.aptos.transaction.testing1.v1.MoveStruct.deserializeBinaryFromReader = function(msg, reader) {
  while (reader.nextField()) {
    if (reader.isEndGroup()) {
      break;
    }
    var field = reader.getFieldNumber();
    switch (field) {
    case 1:
      var value = /** @type {string} */ (reader.readString());
      msg.setName(value);
      break;
    case 2:
      var value = /** @type {boolean} */ (reader.readBool());
      msg.setIsNative(value);
      break;
    case 3:
      var values = /** @type {!Array<!proto.aptos.transaction.testing1.v1.MoveAbility>} */ (reader.isDelimited() ? reader.readPackedEnum() : [reader.readEnum()]);
      for (var i = 0; i < values.length; i++) {
        msg.addAbilities(values[i]);
      }
      break;
    case 4:
      var value = new proto.aptos.transaction.testing1.v1.MoveStructGenericTypeParam;
      reader.readMessage(value,proto.aptos.transaction.testing1.v1.MoveStructGenericTypeParam.deserializeBinaryFromReader);
      msg.addGenericTypeParams(value);
      break;
    case 5:
      var value = new proto.aptos.transaction.testing1.v1.MoveStructField;
      reader.readMessage(value,proto.aptos.transaction.testing1.v1.MoveStructField.deserializeBinaryFromReader);
      msg.addFields(value);
      break;
    default:
      reader.skipField();
      break;
    }
  }
  return msg;
};


/**
 * Serializes the message to binary data (in protobuf wire format).
 * @return {!Uint8Array}
 */
proto.aptos.transaction.testing1.v1.MoveStruct.prototype.serializeBinary = function() {
  var writer = new jspb.BinaryWriter();
  proto.aptos.transaction.testing1.v1.MoveStruct.serializeBinaryToWriter(this, writer);
  return writer.getResultBuffer();
};


/**
 * Serializes the given message to binary data (in protobuf wire
 * format), writing to the given BinaryWriter.
 * @param {!proto.aptos.transaction.testing1.v1.MoveStruct} message
 * @param {!jspb.BinaryWriter} writer
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.aptos.transaction.testing1.v1.MoveStruct.serializeBinaryToWriter = function(message, writer) {
  var f = undefined;
  f = message.getName();
  if (f.length > 0) {
    writer.writeString(
      1,
      f
    );
  }
  f = message.getIsNative();
  if (f) {
    writer.writeBool(
      2,
      f
    );
  }
  f = message.getAbilitiesList();
  if (f.length > 0) {
    writer.writePackedEnum(
      3,
      f
    );
  }
  f = message.getGenericTypeParamsList();
  if (f.length > 0) {
    writer.writeRepeatedMessage(
      4,
      f,
      proto.aptos.transaction.testing1.v1.MoveStructGenericTypeParam.serializeBinaryToWriter
    );
  }
  f = message.getFieldsList();
  if (f.length > 0) {
    writer.writeRepeatedMessage(
      5,
      f,
      proto.aptos.transaction.testing1.v1.MoveStructField.serializeBinaryToWriter
    );
  }
};


/**
 * optional string name = 1;
 * @return {string}
 */
proto.aptos.transaction.testing1.v1.MoveStruct.prototype.getName = function() {
  return /** @type {string} */ (jspb.Message.getFieldWithDefault(this, 1, ""));
};


/**
 * @param {string} value
 * @return {!proto.aptos.transaction.testing1.v1.MoveStruct} returns this
 */
proto.aptos.transaction.testing1.v1.MoveStruct.prototype.setName = function(value) {
  return jspb.Message.setProto3StringField(this, 1, value);
};


/**
 * optional bool is_native = 2;
 * @return {boolean}
 */
proto.aptos.transaction.testing1.v1.MoveStruct.prototype.getIsNative = function() {
  return /** @type {boolean} */ (jspb.Message.getBooleanFieldWithDefault(this, 2, false));
};


/**
 * @param {boolean} value
 * @return {!proto.aptos.transaction.testing1.v1.MoveStruct} returns this
 */
proto.aptos.transaction.testing1.v1.MoveStruct.prototype.setIsNative = function(value) {
  return jspb.Message.setProto3BooleanField(this, 2, value);
};


/**
 * repeated MoveAbility abilities = 3;
 * @return {!Array<!proto.aptos.transaction.testing1.v1.MoveAbility>}
 */
proto.aptos.transaction.testing1.v1.MoveStruct.prototype.getAbilitiesList = function() {
  return /** @type {!Array<!proto.aptos.transaction.testing1.v1.MoveAbility>} */ (jspb.Message.getRepeatedField(this, 3));
};


/**
 * @param {!Array<!proto.aptos.transaction.testing1.v1.MoveAbility>} value
 * @return {!proto.aptos.transaction.testing1.v1.MoveStruct} returns this
 */
proto.aptos.transaction.testing1.v1.MoveStruct.prototype.setAbilitiesList = function(value) {
  return jspb.Message.setField(this, 3, value || []);
};


/**
 * @param {!proto.aptos.transaction.testing1.v1.MoveAbility} value
 * @param {number=} opt_index
 * @return {!proto.aptos.transaction.testing1.v1.MoveStruct} returns this
 */
proto.aptos.transaction.testing1.v1.MoveStruct.prototype.addAbilities = function(value, opt_index) {
  return jspb.Message.addToRepeatedField(this, 3, value, opt_index);
};


/**
 * Clears the list making it empty but non-null.
 * @return {!proto.aptos.transaction.testing1.v1.MoveStruct} returns this
 */
proto.aptos.transaction.testing1.v1.MoveStruct.prototype.clearAbilitiesList = function() {
  return this.setAbilitiesList([]);
};


/**
 * repeated MoveStructGenericTypeParam generic_type_params = 4;
 * @return {!Array<!proto.aptos.transaction.testing1.v1.MoveStructGenericTypeParam>}
 */
proto.aptos.transaction.testing1.v1.MoveStruct.prototype.getGenericTypeParamsList = function() {
  return /** @type{!Array<!proto.aptos.transaction.testing1.v1.MoveStructGenericTypeParam>} */ (
    jspb.Message.getRepeatedWrapperField(this, proto.aptos.transaction.testing1.v1.MoveStructGenericTypeParam, 4));
};


/**
 * @param {!Array<!proto.aptos.transaction.testing1.v1.MoveStructGenericTypeParam>} value
 * @return {!proto.aptos.transaction.testing1.v1.MoveStruct} returns this
*/
proto.aptos.transaction.testing1.v1.MoveStruct.prototype.setGenericTypeParamsList = function(value) {
  return jspb.Message.setRepeatedWrapperField(this, 4, value);
};


/**
 * @param {!proto.aptos.transaction.testing1.v1.MoveStructGenericTypeParam=} opt_value
 * @param {number=} opt_index
 * @return {!proto.aptos.transaction.testing1.v1.MoveStructGenericTypeParam}
 */
proto.aptos.transaction.testing1.v1.MoveStruct.prototype.addGenericTypeParams = function(opt_value, opt_index) {
  return jspb.Message.addToRepeatedWrapperField(this, 4, opt_value, proto.aptos.transaction.testing1.v1.MoveStructGenericTypeParam, opt_index);
};


/**
 * Clears the list making it empty but non-null.
 * @return {!proto.aptos.transaction.testing1.v1.MoveStruct} returns this
 */
proto.aptos.transaction.testing1.v1.MoveStruct.prototype.clearGenericTypeParamsList = function() {
  return this.setGenericTypeParamsList([]);
};


/**
 * repeated MoveStructField fields = 5;
 * @return {!Array<!proto.aptos.transaction.testing1.v1.MoveStructField>}
 */
proto.aptos.transaction.testing1.v1.MoveStruct.prototype.getFieldsList = function() {
  return /** @type{!Array<!proto.aptos.transaction.testing1.v1.MoveStructField>} */ (
    jspb.Message.getRepeatedWrapperField(this, proto.aptos.transaction.testing1.v1.MoveStructField, 5));
};


/**
 * @param {!Array<!proto.aptos.transaction.testing1.v1.MoveStructField>} value
 * @return {!proto.aptos.transaction.testing1.v1.MoveStruct} returns this
*/
proto.aptos.transaction.testing1.v1.MoveStruct.prototype.setFieldsList = function(value) {
  return jspb.Message.setRepeatedWrapperField(this, 5, value);
};


/**
 * @param {!proto.aptos.transaction.testing1.v1.MoveStructField=} opt_value
 * @param {number=} opt_index
 * @return {!proto.aptos.transaction.testing1.v1.MoveStructField}
 */
proto.aptos.transaction.testing1.v1.MoveStruct.prototype.addFields = function(opt_value, opt_index) {
  return jspb.Message.addToRepeatedWrapperField(this, 5, opt_value, proto.aptos.transaction.testing1.v1.MoveStructField, opt_index);
};


/**
 * Clears the list making it empty but non-null.
 * @return {!proto.aptos.transaction.testing1.v1.MoveStruct} returns this
 */
proto.aptos.transaction.testing1.v1.MoveStruct.prototype.clearFieldsList = function() {
  return this.setFieldsList([]);
};



/**
 * List of repeated fields within this message type.
 * @private {!Array<number>}
 * @const
 */
proto.aptos.transaction.testing1.v1.MoveStructGenericTypeParam.repeatedFields_ = [1];



if (jspb.Message.GENERATE_TO_OBJECT) {
/**
 * Creates an object representation of this proto.
 * Field names that are reserved in JavaScript and will be renamed to pb_name.
 * Optional fields that are not set will be set to undefined.
 * To access a reserved field use, foo.pb_<name>, eg, foo.pb_default.
 * For the list of reserved names please see:
 *     net/proto2/compiler/js/internal/generator.cc#kKeyword.
 * @param {boolean=} opt_includeInstance Deprecated. whether to include the
 *     JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @return {!Object}
 */
proto.aptos.transaction.testing1.v1.MoveStructGenericTypeParam.prototype.toObject = function(opt_includeInstance) {
  return proto.aptos.transaction.testing1.v1.MoveStructGenericTypeParam.toObject(opt_includeInstance, this);
};


/**
 * Static version of the {@see toObject} method.
 * @param {boolean|undefined} includeInstance Deprecated. Whether to include
 *     the JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @param {!proto.aptos.transaction.testing1.v1.MoveStructGenericTypeParam} msg The msg instance to transform.
 * @return {!Object}
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.aptos.transaction.testing1.v1.MoveStructGenericTypeParam.toObject = function(includeInstance, msg) {
  var f, obj = {
    constraintsList: (f = jspb.Message.getRepeatedField(msg, 1)) == null ? undefined : f,
    isPhantom: jspb.Message.getBooleanFieldWithDefault(msg, 2, false)
  };

  if (includeInstance) {
    obj.$jspbMessageInstance = msg;
  }
  return obj;
};
}


/**
 * Deserializes binary data (in protobuf wire format).
 * @param {jspb.ByteSource} bytes The bytes to deserialize.
 * @return {!proto.aptos.transaction.testing1.v1.MoveStructGenericTypeParam}
 */
proto.aptos.transaction.testing1.v1.MoveStructGenericTypeParam.deserializeBinary = function(bytes) {
  var reader = new jspb.BinaryReader(bytes);
  var msg = new proto.aptos.transaction.testing1.v1.MoveStructGenericTypeParam;
  return proto.aptos.transaction.testing1.v1.MoveStructGenericTypeParam.deserializeBinaryFromReader(msg, reader);
};


/**
 * Deserializes binary data (in protobuf wire format) from the
 * given reader into the given message object.
 * @param {!proto.aptos.transaction.testing1.v1.MoveStructGenericTypeParam} msg The message object to deserialize into.
 * @param {!jspb.BinaryReader} reader The BinaryReader to use.
 * @return {!proto.aptos.transaction.testing1.v1.MoveStructGenericTypeParam}
 */
proto.aptos.transaction.testing1.v1.MoveStructGenericTypeParam.deserializeBinaryFromReader = function(msg, reader) {
  while (reader.nextField()) {
    if (reader.isEndGroup()) {
      break;
    }
    var field = reader.getFieldNumber();
    switch (field) {
    case 1:
      var values = /** @type {!Array<!proto.aptos.transaction.testing1.v1.MoveAbility>} */ (reader.isDelimited() ? reader.readPackedEnum() : [reader.readEnum()]);
      for (var i = 0; i < values.length; i++) {
        msg.addConstraints(values[i]);
      }
      break;
    case 2:
      var value = /** @type {boolean} */ (reader.readBool());
      msg.setIsPhantom(value);
      break;
    default:
      reader.skipField();
      break;
    }
  }
  return msg;
};


/**
 * Serializes the message to binary data (in protobuf wire format).
 * @return {!Uint8Array}
 */
proto.aptos.transaction.testing1.v1.MoveStructGenericTypeParam.prototype.serializeBinary = function() {
  var writer = new jspb.BinaryWriter();
  proto.aptos.transaction.testing1.v1.MoveStructGenericTypeParam.serializeBinaryToWriter(this, writer);
  return writer.getResultBuffer();
};


/**
 * Serializes the given message to binary data (in protobuf wire
 * format), writing to the given BinaryWriter.
 * @param {!proto.aptos.transaction.testing1.v1.MoveStructGenericTypeParam} message
 * @param {!jspb.BinaryWriter} writer
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.aptos.transaction.testing1.v1.MoveStructGenericTypeParam.serializeBinaryToWriter = function(message, writer) {
  var f = undefined;
  f = message.getConstraintsList();
  if (f.length > 0) {
    writer.writePackedEnum(
      1,
      f
    );
  }
  f = message.getIsPhantom();
  if (f) {
    writer.writeBool(
      2,
      f
    );
  }
};


/**
 * repeated MoveAbility constraints = 1;
 * @return {!Array<!proto.aptos.transaction.testing1.v1.MoveAbility>}
 */
proto.aptos.transaction.testing1.v1.MoveStructGenericTypeParam.prototype.getConstraintsList = function() {
  return /** @type {!Array<!proto.aptos.transaction.testing1.v1.MoveAbility>} */ (jspb.Message.getRepeatedField(this, 1));
};


/**
 * @param {!Array<!proto.aptos.transaction.testing1.v1.MoveAbility>} value
 * @return {!proto.aptos.transaction.testing1.v1.MoveStructGenericTypeParam} returns this
 */
proto.aptos.transaction.testing1.v1.MoveStructGenericTypeParam.prototype.setConstraintsList = function(value) {
  return jspb.Message.setField(this, 1, value || []);
};


/**
 * @param {!proto.aptos.transaction.testing1.v1.MoveAbility} value
 * @param {number=} opt_index
 * @return {!proto.aptos.transaction.testing1.v1.MoveStructGenericTypeParam} returns this
 */
proto.aptos.transaction.testing1.v1.MoveStructGenericTypeParam.prototype.addConstraints = function(value, opt_index) {
  return jspb.Message.addToRepeatedField(this, 1, value, opt_index);
};


/**
 * Clears the list making it empty but non-null.
 * @return {!proto.aptos.transaction.testing1.v1.MoveStructGenericTypeParam} returns this
 */
proto.aptos.transaction.testing1.v1.MoveStructGenericTypeParam.prototype.clearConstraintsList = function() {
  return this.setConstraintsList([]);
};


/**
 * optional bool is_phantom = 2;
 * @return {boolean}
 */
proto.aptos.transaction.testing1.v1.MoveStructGenericTypeParam.prototype.getIsPhantom = function() {
  return /** @type {boolean} */ (jspb.Message.getBooleanFieldWithDefault(this, 2, false));
};


/**
 * @param {boolean} value
 * @return {!proto.aptos.transaction.testing1.v1.MoveStructGenericTypeParam} returns this
 */
proto.aptos.transaction.testing1.v1.MoveStructGenericTypeParam.prototype.setIsPhantom = function(value) {
  return jspb.Message.setProto3BooleanField(this, 2, value);
};





if (jspb.Message.GENERATE_TO_OBJECT) {
/**
 * Creates an object representation of this proto.
 * Field names that are reserved in JavaScript and will be renamed to pb_name.
 * Optional fields that are not set will be set to undefined.
 * To access a reserved field use, foo.pb_<name>, eg, foo.pb_default.
 * For the list of reserved names please see:
 *     net/proto2/compiler/js/internal/generator.cc#kKeyword.
 * @param {boolean=} opt_includeInstance Deprecated. whether to include the
 *     JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @return {!Object}
 */
proto.aptos.transaction.testing1.v1.MoveStructField.prototype.toObject = function(opt_includeInstance) {
  return proto.aptos.transaction.testing1.v1.MoveStructField.toObject(opt_includeInstance, this);
};


/**
 * Static version of the {@see toObject} method.
 * @param {boolean|undefined} includeInstance Deprecated. Whether to include
 *     the JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @param {!proto.aptos.transaction.testing1.v1.MoveStructField} msg The msg instance to transform.
 * @return {!Object}
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.aptos.transaction.testing1.v1.MoveStructField.toObject = function(includeInstance, msg) {
  var f, obj = {
    name: jspb.Message.getFieldWithDefault(msg, 1, ""),
    type: (f = msg.getType()) && proto.aptos.transaction.testing1.v1.MoveType.toObject(includeInstance, f)
  };

  if (includeInstance) {
    obj.$jspbMessageInstance = msg;
  }
  return obj;
};
}


/**
 * Deserializes binary data (in protobuf wire format).
 * @param {jspb.ByteSource} bytes The bytes to deserialize.
 * @return {!proto.aptos.transaction.testing1.v1.MoveStructField}
 */
proto.aptos.transaction.testing1.v1.MoveStructField.deserializeBinary = function(bytes) {
  var reader = new jspb.BinaryReader(bytes);
  var msg = new proto.aptos.transaction.testing1.v1.MoveStructField;
  return proto.aptos.transaction.testing1.v1.MoveStructField.deserializeBinaryFromReader(msg, reader);
};


/**
 * Deserializes binary data (in protobuf wire format) from the
 * given reader into the given message object.
 * @param {!proto.aptos.transaction.testing1.v1.MoveStructField} msg The message object to deserialize into.
 * @param {!jspb.BinaryReader} reader The BinaryReader to use.
 * @return {!proto.aptos.transaction.testing1.v1.MoveStructField}
 */
proto.aptos.transaction.testing1.v1.MoveStructField.deserializeBinaryFromReader = function(msg, reader) {
  while (reader.nextField()) {
    if (reader.isEndGroup()) {
      break;
    }
    var field = reader.getFieldNumber();
    switch (field) {
    case 1:
      var value = /** @type {string} */ (reader.readString());
      msg.setName(value);
      break;
    case 2:
      var value = new proto.aptos.transaction.testing1.v1.MoveType;
      reader.readMessage(value,proto.aptos.transaction.testing1.v1.MoveType.deserializeBinaryFromReader);
      msg.setType(value);
      break;
    default:
      reader.skipField();
      break;
    }
  }
  return msg;
};


/**
 * Serializes the message to binary data (in protobuf wire format).
 * @return {!Uint8Array}
 */
proto.aptos.transaction.testing1.v1.MoveStructField.prototype.serializeBinary = function() {
  var writer = new jspb.BinaryWriter();
  proto.aptos.transaction.testing1.v1.MoveStructField.serializeBinaryToWriter(this, writer);
  return writer.getResultBuffer();
};


/**
 * Serializes the given message to binary data (in protobuf wire
 * format), writing to the given BinaryWriter.
 * @param {!proto.aptos.transaction.testing1.v1.MoveStructField} message
 * @param {!jspb.BinaryWriter} writer
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.aptos.transaction.testing1.v1.MoveStructField.serializeBinaryToWriter = function(message, writer) {
  var f = undefined;
  f = message.getName();
  if (f.length > 0) {
    writer.writeString(
      1,
      f
    );
  }
  f = message.getType();
  if (f != null) {
    writer.writeMessage(
      2,
      f,
      proto.aptos.transaction.testing1.v1.MoveType.serializeBinaryToWriter
    );
  }
};


/**
 * optional string name = 1;
 * @return {string}
 */
proto.aptos.transaction.testing1.v1.MoveStructField.prototype.getName = function() {
  return /** @type {string} */ (jspb.Message.getFieldWithDefault(this, 1, ""));
};


/**
 * @param {string} value
 * @return {!proto.aptos.transaction.testing1.v1.MoveStructField} returns this
 */
proto.aptos.transaction.testing1.v1.MoveStructField.prototype.setName = function(value) {
  return jspb.Message.setProto3StringField(this, 1, value);
};


/**
 * optional MoveType type = 2;
 * @return {?proto.aptos.transaction.testing1.v1.MoveType}
 */
proto.aptos.transaction.testing1.v1.MoveStructField.prototype.getType = function() {
  return /** @type{?proto.aptos.transaction.testing1.v1.MoveType} */ (
    jspb.Message.getWrapperField(this, proto.aptos.transaction.testing1.v1.MoveType, 2));
};


/**
 * @param {?proto.aptos.transaction.testing1.v1.MoveType|undefined} value
 * @return {!proto.aptos.transaction.testing1.v1.MoveStructField} returns this
*/
proto.aptos.transaction.testing1.v1.MoveStructField.prototype.setType = function(value) {
  return jspb.Message.setWrapperField(this, 2, value);
};


/**
 * Clears the message field making it undefined.
 * @return {!proto.aptos.transaction.testing1.v1.MoveStructField} returns this
 */
proto.aptos.transaction.testing1.v1.MoveStructField.prototype.clearType = function() {
  return this.setType(undefined);
};


/**
 * Returns whether this field is set.
 * @return {boolean}
 */
proto.aptos.transaction.testing1.v1.MoveStructField.prototype.hasType = function() {
  return jspb.Message.getField(this, 2) != null;
};



/**
 * List of repeated fields within this message type.
 * @private {!Array<number>}
 * @const
 */
proto.aptos.transaction.testing1.v1.MoveFunctionGenericTypeParam.repeatedFields_ = [1];



if (jspb.Message.GENERATE_TO_OBJECT) {
/**
 * Creates an object representation of this proto.
 * Field names that are reserved in JavaScript and will be renamed to pb_name.
 * Optional fields that are not set will be set to undefined.
 * To access a reserved field use, foo.pb_<name>, eg, foo.pb_default.
 * For the list of reserved names please see:
 *     net/proto2/compiler/js/internal/generator.cc#kKeyword.
 * @param {boolean=} opt_includeInstance Deprecated. whether to include the
 *     JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @return {!Object}
 */
proto.aptos.transaction.testing1.v1.MoveFunctionGenericTypeParam.prototype.toObject = function(opt_includeInstance) {
  return proto.aptos.transaction.testing1.v1.MoveFunctionGenericTypeParam.toObject(opt_includeInstance, this);
};


/**
 * Static version of the {@see toObject} method.
 * @param {boolean|undefined} includeInstance Deprecated. Whether to include
 *     the JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @param {!proto.aptos.transaction.testing1.v1.MoveFunctionGenericTypeParam} msg The msg instance to transform.
 * @return {!Object}
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.aptos.transaction.testing1.v1.MoveFunctionGenericTypeParam.toObject = function(includeInstance, msg) {
  var f, obj = {
    constraintsList: (f = jspb.Message.getRepeatedField(msg, 1)) == null ? undefined : f
  };

  if (includeInstance) {
    obj.$jspbMessageInstance = msg;
  }
  return obj;
};
}


/**
 * Deserializes binary data (in protobuf wire format).
 * @param {jspb.ByteSource} bytes The bytes to deserialize.
 * @return {!proto.aptos.transaction.testing1.v1.MoveFunctionGenericTypeParam}
 */
proto.aptos.transaction.testing1.v1.MoveFunctionGenericTypeParam.deserializeBinary = function(bytes) {
  var reader = new jspb.BinaryReader(bytes);
  var msg = new proto.aptos.transaction.testing1.v1.MoveFunctionGenericTypeParam;
  return proto.aptos.transaction.testing1.v1.MoveFunctionGenericTypeParam.deserializeBinaryFromReader(msg, reader);
};


/**
 * Deserializes binary data (in protobuf wire format) from the
 * given reader into the given message object.
 * @param {!proto.aptos.transaction.testing1.v1.MoveFunctionGenericTypeParam} msg The message object to deserialize into.
 * @param {!jspb.BinaryReader} reader The BinaryReader to use.
 * @return {!proto.aptos.transaction.testing1.v1.MoveFunctionGenericTypeParam}
 */
proto.aptos.transaction.testing1.v1.MoveFunctionGenericTypeParam.deserializeBinaryFromReader = function(msg, reader) {
  while (reader.nextField()) {
    if (reader.isEndGroup()) {
      break;
    }
    var field = reader.getFieldNumber();
    switch (field) {
    case 1:
      var values = /** @type {!Array<!proto.aptos.transaction.testing1.v1.MoveAbility>} */ (reader.isDelimited() ? reader.readPackedEnum() : [reader.readEnum()]);
      for (var i = 0; i < values.length; i++) {
        msg.addConstraints(values[i]);
      }
      break;
    default:
      reader.skipField();
      break;
    }
  }
  return msg;
};


/**
 * Serializes the message to binary data (in protobuf wire format).
 * @return {!Uint8Array}
 */
proto.aptos.transaction.testing1.v1.MoveFunctionGenericTypeParam.prototype.serializeBinary = function() {
  var writer = new jspb.BinaryWriter();
  proto.aptos.transaction.testing1.v1.MoveFunctionGenericTypeParam.serializeBinaryToWriter(this, writer);
  return writer.getResultBuffer();
};


/**
 * Serializes the given message to binary data (in protobuf wire
 * format), writing to the given BinaryWriter.
 * @param {!proto.aptos.transaction.testing1.v1.MoveFunctionGenericTypeParam} message
 * @param {!jspb.BinaryWriter} writer
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.aptos.transaction.testing1.v1.MoveFunctionGenericTypeParam.serializeBinaryToWriter = function(message, writer) {
  var f = undefined;
  f = message.getConstraintsList();
  if (f.length > 0) {
    writer.writePackedEnum(
      1,
      f
    );
  }
};


/**
 * repeated MoveAbility constraints = 1;
 * @return {!Array<!proto.aptos.transaction.testing1.v1.MoveAbility>}
 */
proto.aptos.transaction.testing1.v1.MoveFunctionGenericTypeParam.prototype.getConstraintsList = function() {
  return /** @type {!Array<!proto.aptos.transaction.testing1.v1.MoveAbility>} */ (jspb.Message.getRepeatedField(this, 1));
};


/**
 * @param {!Array<!proto.aptos.transaction.testing1.v1.MoveAbility>} value
 * @return {!proto.aptos.transaction.testing1.v1.MoveFunctionGenericTypeParam} returns this
 */
proto.aptos.transaction.testing1.v1.MoveFunctionGenericTypeParam.prototype.setConstraintsList = function(value) {
  return jspb.Message.setField(this, 1, value || []);
};


/**
 * @param {!proto.aptos.transaction.testing1.v1.MoveAbility} value
 * @param {number=} opt_index
 * @return {!proto.aptos.transaction.testing1.v1.MoveFunctionGenericTypeParam} returns this
 */
proto.aptos.transaction.testing1.v1.MoveFunctionGenericTypeParam.prototype.addConstraints = function(value, opt_index) {
  return jspb.Message.addToRepeatedField(this, 1, value, opt_index);
};


/**
 * Clears the list making it empty but non-null.
 * @return {!proto.aptos.transaction.testing1.v1.MoveFunctionGenericTypeParam} returns this
 */
proto.aptos.transaction.testing1.v1.MoveFunctionGenericTypeParam.prototype.clearConstraintsList = function() {
  return this.setConstraintsList([]);
};



/**
 * Oneof group definitions for this message. Each group defines the field
 * numbers belonging to that group. When of these fields' value is set, all
 * other fields in the group are cleared. During deserialization, if multiple
 * fields are encountered for a group, only the last value seen will be kept.
 * @private {!Array<!Array<number>>}
 * @const
 */
proto.aptos.transaction.testing1.v1.MoveType.oneofGroups_ = [[3,4,5,6,7]];

/**
 * @enum {number}
 */
proto.aptos.transaction.testing1.v1.MoveType.ContentCase = {
  CONTENT_NOT_SET: 0,
  VECTOR: 3,
  STRUCT: 4,
  GENERIC_TYPE_PARAM_INDEX: 5,
  REFERENCE: 6,
  UNPARSABLE: 7
};

/**
 * @return {proto.aptos.transaction.testing1.v1.MoveType.ContentCase}
 */
proto.aptos.transaction.testing1.v1.MoveType.prototype.getContentCase = function() {
  return /** @type {proto.aptos.transaction.testing1.v1.MoveType.ContentCase} */(jspb.Message.computeOneofCase(this, proto.aptos.transaction.testing1.v1.MoveType.oneofGroups_[0]));
};



if (jspb.Message.GENERATE_TO_OBJECT) {
/**
 * Creates an object representation of this proto.
 * Field names that are reserved in JavaScript and will be renamed to pb_name.
 * Optional fields that are not set will be set to undefined.
 * To access a reserved field use, foo.pb_<name>, eg, foo.pb_default.
 * For the list of reserved names please see:
 *     net/proto2/compiler/js/internal/generator.cc#kKeyword.
 * @param {boolean=} opt_includeInstance Deprecated. whether to include the
 *     JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @return {!Object}
 */
proto.aptos.transaction.testing1.v1.MoveType.prototype.toObject = function(opt_includeInstance) {
  return proto.aptos.transaction.testing1.v1.MoveType.toObject(opt_includeInstance, this);
};


/**
 * Static version of the {@see toObject} method.
 * @param {boolean|undefined} includeInstance Deprecated. Whether to include
 *     the JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @param {!proto.aptos.transaction.testing1.v1.MoveType} msg The msg instance to transform.
 * @return {!Object}
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.aptos.transaction.testing1.v1.MoveType.toObject = function(includeInstance, msg) {
  var f, obj = {
    type: jspb.Message.getFieldWithDefault(msg, 1, 0),
    vector: (f = msg.getVector()) && proto.aptos.transaction.testing1.v1.MoveType.toObject(includeInstance, f),
    struct: (f = msg.getStruct()) && proto.aptos.transaction.testing1.v1.MoveStructTag.toObject(includeInstance, f),
    genericTypeParamIndex: jspb.Message.getFieldWithDefault(msg, 5, 0),
    reference: (f = msg.getReference()) && proto.aptos.transaction.testing1.v1.MoveType.ReferenceType.toObject(includeInstance, f),
    unparsable: jspb.Message.getFieldWithDefault(msg, 7, "")
  };

  if (includeInstance) {
    obj.$jspbMessageInstance = msg;
  }
  return obj;
};
}


/**
 * Deserializes binary data (in protobuf wire format).
 * @param {jspb.ByteSource} bytes The bytes to deserialize.
 * @return {!proto.aptos.transaction.testing1.v1.MoveType}
 */
proto.aptos.transaction.testing1.v1.MoveType.deserializeBinary = function(bytes) {
  var reader = new jspb.BinaryReader(bytes);
  var msg = new proto.aptos.transaction.testing1.v1.MoveType;
  return proto.aptos.transaction.testing1.v1.MoveType.deserializeBinaryFromReader(msg, reader);
};


/**
 * Deserializes binary data (in protobuf wire format) from the
 * given reader into the given message object.
 * @param {!proto.aptos.transaction.testing1.v1.MoveType} msg The message object to deserialize into.
 * @param {!jspb.BinaryReader} reader The BinaryReader to use.
 * @return {!proto.aptos.transaction.testing1.v1.MoveType}
 */
proto.aptos.transaction.testing1.v1.MoveType.deserializeBinaryFromReader = function(msg, reader) {
  while (reader.nextField()) {
    if (reader.isEndGroup()) {
      break;
    }
    var field = reader.getFieldNumber();
    switch (field) {
    case 1:
      var value = /** @type {!proto.aptos.transaction.testing1.v1.MoveTypes} */ (reader.readEnum());
      msg.setType(value);
      break;
    case 3:
      var value = new proto.aptos.transaction.testing1.v1.MoveType;
      reader.readMessage(value,proto.aptos.transaction.testing1.v1.MoveType.deserializeBinaryFromReader);
      msg.setVector(value);
      break;
    case 4:
      var value = new proto.aptos.transaction.testing1.v1.MoveStructTag;
      reader.readMessage(value,proto.aptos.transaction.testing1.v1.MoveStructTag.deserializeBinaryFromReader);
      msg.setStruct(value);
      break;
    case 5:
      var value = /** @type {number} */ (reader.readUint32());
      msg.setGenericTypeParamIndex(value);
      break;
    case 6:
      var value = new proto.aptos.transaction.testing1.v1.MoveType.ReferenceType;
      reader.readMessage(value,proto.aptos.transaction.testing1.v1.MoveType.ReferenceType.deserializeBinaryFromReader);
      msg.setReference(value);
      break;
    case 7:
      var value = /** @type {string} */ (reader.readString());
      msg.setUnparsable(value);
      break;
    default:
      reader.skipField();
      break;
    }
  }
  return msg;
};


/**
 * Serializes the message to binary data (in protobuf wire format).
 * @return {!Uint8Array}
 */
proto.aptos.transaction.testing1.v1.MoveType.prototype.serializeBinary = function() {
  var writer = new jspb.BinaryWriter();
  proto.aptos.transaction.testing1.v1.MoveType.serializeBinaryToWriter(this, writer);
  return writer.getResultBuffer();
};


/**
 * Serializes the given message to binary data (in protobuf wire
 * format), writing to the given BinaryWriter.
 * @param {!proto.aptos.transaction.testing1.v1.MoveType} message
 * @param {!jspb.BinaryWriter} writer
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.aptos.transaction.testing1.v1.MoveType.serializeBinaryToWriter = function(message, writer) {
  var f = undefined;
  f = message.getType();
  if (f !== 0.0) {
    writer.writeEnum(
      1,
      f
    );
  }
  f = message.getVector();
  if (f != null) {
    writer.writeMessage(
      3,
      f,
      proto.aptos.transaction.testing1.v1.MoveType.serializeBinaryToWriter
    );
  }
  f = message.getStruct();
  if (f != null) {
    writer.writeMessage(
      4,
      f,
      proto.aptos.transaction.testing1.v1.MoveStructTag.serializeBinaryToWriter
    );
  }
  f = /** @type {number} */ (jspb.Message.getField(message, 5));
  if (f != null) {
    writer.writeUint32(
      5,
      f
    );
  }
  f = message.getReference();
  if (f != null) {
    writer.writeMessage(
      6,
      f,
      proto.aptos.transaction.testing1.v1.MoveType.ReferenceType.serializeBinaryToWriter
    );
  }
  f = /** @type {string} */ (jspb.Message.getField(message, 7));
  if (f != null) {
    writer.writeString(
      7,
      f
    );
  }
};





if (jspb.Message.GENERATE_TO_OBJECT) {
/**
 * Creates an object representation of this proto.
 * Field names that are reserved in JavaScript and will be renamed to pb_name.
 * Optional fields that are not set will be set to undefined.
 * To access a reserved field use, foo.pb_<name>, eg, foo.pb_default.
 * For the list of reserved names please see:
 *     net/proto2/compiler/js/internal/generator.cc#kKeyword.
 * @param {boolean=} opt_includeInstance Deprecated. whether to include the
 *     JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @return {!Object}
 */
proto.aptos.transaction.testing1.v1.MoveType.ReferenceType.prototype.toObject = function(opt_includeInstance) {
  return proto.aptos.transaction.testing1.v1.MoveType.ReferenceType.toObject(opt_includeInstance, this);
};


/**
 * Static version of the {@see toObject} method.
 * @param {boolean|undefined} includeInstance Deprecated. Whether to include
 *     the JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @param {!proto.aptos.transaction.testing1.v1.MoveType.ReferenceType} msg The msg instance to transform.
 * @return {!Object}
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.aptos.transaction.testing1.v1.MoveType.ReferenceType.toObject = function(includeInstance, msg) {
  var f, obj = {
    mutable: jspb.Message.getBooleanFieldWithDefault(msg, 1, false),
    to: (f = msg.getTo()) && proto.aptos.transaction.testing1.v1.MoveType.toObject(includeInstance, f)
  };

  if (includeInstance) {
    obj.$jspbMessageInstance = msg;
  }
  return obj;
};
}


/**
 * Deserializes binary data (in protobuf wire format).
 * @param {jspb.ByteSource} bytes The bytes to deserialize.
 * @return {!proto.aptos.transaction.testing1.v1.MoveType.ReferenceType}
 */
proto.aptos.transaction.testing1.v1.MoveType.ReferenceType.deserializeBinary = function(bytes) {
  var reader = new jspb.BinaryReader(bytes);
  var msg = new proto.aptos.transaction.testing1.v1.MoveType.ReferenceType;
  return proto.aptos.transaction.testing1.v1.MoveType.ReferenceType.deserializeBinaryFromReader(msg, reader);
};


/**
 * Deserializes binary data (in protobuf wire format) from the
 * given reader into the given message object.
 * @param {!proto.aptos.transaction.testing1.v1.MoveType.ReferenceType} msg The message object to deserialize into.
 * @param {!jspb.BinaryReader} reader The BinaryReader to use.
 * @return {!proto.aptos.transaction.testing1.v1.MoveType.ReferenceType}
 */
proto.aptos.transaction.testing1.v1.MoveType.ReferenceType.deserializeBinaryFromReader = function(msg, reader) {
  while (reader.nextField()) {
    if (reader.isEndGroup()) {
      break;
    }
    var field = reader.getFieldNumber();
    switch (field) {
    case 1:
      var value = /** @type {boolean} */ (reader.readBool());
      msg.setMutable(value);
      break;
    case 2:
      var value = new proto.aptos.transaction.testing1.v1.MoveType;
      reader.readMessage(value,proto.aptos.transaction.testing1.v1.MoveType.deserializeBinaryFromReader);
      msg.setTo(value);
      break;
    default:
      reader.skipField();
      break;
    }
  }
  return msg;
};


/**
 * Serializes the message to binary data (in protobuf wire format).
 * @return {!Uint8Array}
 */
proto.aptos.transaction.testing1.v1.MoveType.ReferenceType.prototype.serializeBinary = function() {
  var writer = new jspb.BinaryWriter();
  proto.aptos.transaction.testing1.v1.MoveType.ReferenceType.serializeBinaryToWriter(this, writer);
  return writer.getResultBuffer();
};


/**
 * Serializes the given message to binary data (in protobuf wire
 * format), writing to the given BinaryWriter.
 * @param {!proto.aptos.transaction.testing1.v1.MoveType.ReferenceType} message
 * @param {!jspb.BinaryWriter} writer
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.aptos.transaction.testing1.v1.MoveType.ReferenceType.serializeBinaryToWriter = function(message, writer) {
  var f = undefined;
  f = message.getMutable();
  if (f) {
    writer.writeBool(
      1,
      f
    );
  }
  f = message.getTo();
  if (f != null) {
    writer.writeMessage(
      2,
      f,
      proto.aptos.transaction.testing1.v1.MoveType.serializeBinaryToWriter
    );
  }
};


/**
 * optional bool mutable = 1;
 * @return {boolean}
 */
proto.aptos.transaction.testing1.v1.MoveType.ReferenceType.prototype.getMutable = function() {
  return /** @type {boolean} */ (jspb.Message.getBooleanFieldWithDefault(this, 1, false));
};


/**
 * @param {boolean} value
 * @return {!proto.aptos.transaction.testing1.v1.MoveType.ReferenceType} returns this
 */
proto.aptos.transaction.testing1.v1.MoveType.ReferenceType.prototype.setMutable = function(value) {
  return jspb.Message.setProto3BooleanField(this, 1, value);
};


/**
 * optional MoveType to = 2;
 * @return {?proto.aptos.transaction.testing1.v1.MoveType}
 */
proto.aptos.transaction.testing1.v1.MoveType.ReferenceType.prototype.getTo = function() {
  return /** @type{?proto.aptos.transaction.testing1.v1.MoveType} */ (
    jspb.Message.getWrapperField(this, proto.aptos.transaction.testing1.v1.MoveType, 2));
};


/**
 * @param {?proto.aptos.transaction.testing1.v1.MoveType|undefined} value
 * @return {!proto.aptos.transaction.testing1.v1.MoveType.ReferenceType} returns this
*/
proto.aptos.transaction.testing1.v1.MoveType.ReferenceType.prototype.setTo = function(value) {
  return jspb.Message.setWrapperField(this, 2, value);
};


/**
 * Clears the message field making it undefined.
 * @return {!proto.aptos.transaction.testing1.v1.MoveType.ReferenceType} returns this
 */
proto.aptos.transaction.testing1.v1.MoveType.ReferenceType.prototype.clearTo = function() {
  return this.setTo(undefined);
};


/**
 * Returns whether this field is set.
 * @return {boolean}
 */
proto.aptos.transaction.testing1.v1.MoveType.ReferenceType.prototype.hasTo = function() {
  return jspb.Message.getField(this, 2) != null;
};


/**
 * optional MoveTypes type = 1;
 * @return {!proto.aptos.transaction.testing1.v1.MoveTypes}
 */
proto.aptos.transaction.testing1.v1.MoveType.prototype.getType = function() {
  return /** @type {!proto.aptos.transaction.testing1.v1.MoveTypes} */ (jspb.Message.getFieldWithDefault(this, 1, 0));
};


/**
 * @param {!proto.aptos.transaction.testing1.v1.MoveTypes} value
 * @return {!proto.aptos.transaction.testing1.v1.MoveType} returns this
 */
proto.aptos.transaction.testing1.v1.MoveType.prototype.setType = function(value) {
  return jspb.Message.setProto3EnumField(this, 1, value);
};


/**
 * optional MoveType vector = 3;
 * @return {?proto.aptos.transaction.testing1.v1.MoveType}
 */
proto.aptos.transaction.testing1.v1.MoveType.prototype.getVector = function() {
  return /** @type{?proto.aptos.transaction.testing1.v1.MoveType} */ (
    jspb.Message.getWrapperField(this, proto.aptos.transaction.testing1.v1.MoveType, 3));
};


/**
 * @param {?proto.aptos.transaction.testing1.v1.MoveType|undefined} value
 * @return {!proto.aptos.transaction.testing1.v1.MoveType} returns this
*/
proto.aptos.transaction.testing1.v1.MoveType.prototype.setVector = function(value) {
  return jspb.Message.setOneofWrapperField(this, 3, proto.aptos.transaction.testing1.v1.MoveType.oneofGroups_[0], value);
};


/**
 * Clears the message field making it undefined.
 * @return {!proto.aptos.transaction.testing1.v1.MoveType} returns this
 */
proto.aptos.transaction.testing1.v1.MoveType.prototype.clearVector = function() {
  return this.setVector(undefined);
};


/**
 * Returns whether this field is set.
 * @return {boolean}
 */
proto.aptos.transaction.testing1.v1.MoveType.prototype.hasVector = function() {
  return jspb.Message.getField(this, 3) != null;
};


/**
 * optional MoveStructTag struct = 4;
 * @return {?proto.aptos.transaction.testing1.v1.MoveStructTag}
 */
proto.aptos.transaction.testing1.v1.MoveType.prototype.getStruct = function() {
  return /** @type{?proto.aptos.transaction.testing1.v1.MoveStructTag} */ (
    jspb.Message.getWrapperField(this, proto.aptos.transaction.testing1.v1.MoveStructTag, 4));
};


/**
 * @param {?proto.aptos.transaction.testing1.v1.MoveStructTag|undefined} value
 * @return {!proto.aptos.transaction.testing1.v1.MoveType} returns this
*/
proto.aptos.transaction.testing1.v1.MoveType.prototype.setStruct = function(value) {
  return jspb.Message.setOneofWrapperField(this, 4, proto.aptos.transaction.testing1.v1.MoveType.oneofGroups_[0], value);
};


/**
 * Clears the message field making it undefined.
 * @return {!proto.aptos.transaction.testing1.v1.MoveType} returns this
 */
proto.aptos.transaction.testing1.v1.MoveType.prototype.clearStruct = function() {
  return this.setStruct(undefined);
};


/**
 * Returns whether this field is set.
 * @return {boolean}
 */
proto.aptos.transaction.testing1.v1.MoveType.prototype.hasStruct = function() {
  return jspb.Message.getField(this, 4) != null;
};


/**
 * optional uint32 generic_type_param_index = 5;
 * @return {number}
 */
proto.aptos.transaction.testing1.v1.MoveType.prototype.getGenericTypeParamIndex = function() {
  return /** @type {number} */ (jspb.Message.getFieldWithDefault(this, 5, 0));
};


/**
 * @param {number} value
 * @return {!proto.aptos.transaction.testing1.v1.MoveType} returns this
 */
proto.aptos.transaction.testing1.v1.MoveType.prototype.setGenericTypeParamIndex = function(value) {
  return jspb.Message.setOneofField(this, 5, proto.aptos.transaction.testing1.v1.MoveType.oneofGroups_[0], value);
};


/**
 * Clears the field making it undefined.
 * @return {!proto.aptos.transaction.testing1.v1.MoveType} returns this
 */
proto.aptos.transaction.testing1.v1.MoveType.prototype.clearGenericTypeParamIndex = function() {
  return jspb.Message.setOneofField(this, 5, proto.aptos.transaction.testing1.v1.MoveType.oneofGroups_[0], undefined);
};


/**
 * Returns whether this field is set.
 * @return {boolean}
 */
proto.aptos.transaction.testing1.v1.MoveType.prototype.hasGenericTypeParamIndex = function() {
  return jspb.Message.getField(this, 5) != null;
};


/**
 * optional ReferenceType reference = 6;
 * @return {?proto.aptos.transaction.testing1.v1.MoveType.ReferenceType}
 */
proto.aptos.transaction.testing1.v1.MoveType.prototype.getReference = function() {
  return /** @type{?proto.aptos.transaction.testing1.v1.MoveType.ReferenceType} */ (
    jspb.Message.getWrapperField(this, proto.aptos.transaction.testing1.v1.MoveType.ReferenceType, 6));
};


/**
 * @param {?proto.aptos.transaction.testing1.v1.MoveType.ReferenceType|undefined} value
 * @return {!proto.aptos.transaction.testing1.v1.MoveType} returns this
*/
proto.aptos.transaction.testing1.v1.MoveType.prototype.setReference = function(value) {
  return jspb.Message.setOneofWrapperField(this, 6, proto.aptos.transaction.testing1.v1.MoveType.oneofGroups_[0], value);
};


/**
 * Clears the message field making it undefined.
 * @return {!proto.aptos.transaction.testing1.v1.MoveType} returns this
 */
proto.aptos.transaction.testing1.v1.MoveType.prototype.clearReference = function() {
  return this.setReference(undefined);
};


/**
 * Returns whether this field is set.
 * @return {boolean}
 */
proto.aptos.transaction.testing1.v1.MoveType.prototype.hasReference = function() {
  return jspb.Message.getField(this, 6) != null;
};


/**
 * optional string unparsable = 7;
 * @return {string}
 */
proto.aptos.transaction.testing1.v1.MoveType.prototype.getUnparsable = function() {
  return /** @type {string} */ (jspb.Message.getFieldWithDefault(this, 7, ""));
};


/**
 * @param {string} value
 * @return {!proto.aptos.transaction.testing1.v1.MoveType} returns this
 */
proto.aptos.transaction.testing1.v1.MoveType.prototype.setUnparsable = function(value) {
  return jspb.Message.setOneofField(this, 7, proto.aptos.transaction.testing1.v1.MoveType.oneofGroups_[0], value);
};


/**
 * Clears the field making it undefined.
 * @return {!proto.aptos.transaction.testing1.v1.MoveType} returns this
 */
proto.aptos.transaction.testing1.v1.MoveType.prototype.clearUnparsable = function() {
  return jspb.Message.setOneofField(this, 7, proto.aptos.transaction.testing1.v1.MoveType.oneofGroups_[0], undefined);
};


/**
 * Returns whether this field is set.
 * @return {boolean}
 */
proto.aptos.transaction.testing1.v1.MoveType.prototype.hasUnparsable = function() {
  return jspb.Message.getField(this, 7) != null;
};





if (jspb.Message.GENERATE_TO_OBJECT) {
/**
 * Creates an object representation of this proto.
 * Field names that are reserved in JavaScript and will be renamed to pb_name.
 * Optional fields that are not set will be set to undefined.
 * To access a reserved field use, foo.pb_<name>, eg, foo.pb_default.
 * For the list of reserved names please see:
 *     net/proto2/compiler/js/internal/generator.cc#kKeyword.
 * @param {boolean=} opt_includeInstance Deprecated. whether to include the
 *     JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @return {!Object}
 */
proto.aptos.transaction.testing1.v1.WriteSetPayload.prototype.toObject = function(opt_includeInstance) {
  return proto.aptos.transaction.testing1.v1.WriteSetPayload.toObject(opt_includeInstance, this);
};


/**
 * Static version of the {@see toObject} method.
 * @param {boolean|undefined} includeInstance Deprecated. Whether to include
 *     the JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @param {!proto.aptos.transaction.testing1.v1.WriteSetPayload} msg The msg instance to transform.
 * @return {!Object}
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.aptos.transaction.testing1.v1.WriteSetPayload.toObject = function(includeInstance, msg) {
  var f, obj = {
    writeSet: (f = msg.getWriteSet()) && proto.aptos.transaction.testing1.v1.WriteSet.toObject(includeInstance, f)
  };

  if (includeInstance) {
    obj.$jspbMessageInstance = msg;
  }
  return obj;
};
}


/**
 * Deserializes binary data (in protobuf wire format).
 * @param {jspb.ByteSource} bytes The bytes to deserialize.
 * @return {!proto.aptos.transaction.testing1.v1.WriteSetPayload}
 */
proto.aptos.transaction.testing1.v1.WriteSetPayload.deserializeBinary = function(bytes) {
  var reader = new jspb.BinaryReader(bytes);
  var msg = new proto.aptos.transaction.testing1.v1.WriteSetPayload;
  return proto.aptos.transaction.testing1.v1.WriteSetPayload.deserializeBinaryFromReader(msg, reader);
};


/**
 * Deserializes binary data (in protobuf wire format) from the
 * given reader into the given message object.
 * @param {!proto.aptos.transaction.testing1.v1.WriteSetPayload} msg The message object to deserialize into.
 * @param {!jspb.BinaryReader} reader The BinaryReader to use.
 * @return {!proto.aptos.transaction.testing1.v1.WriteSetPayload}
 */
proto.aptos.transaction.testing1.v1.WriteSetPayload.deserializeBinaryFromReader = function(msg, reader) {
  while (reader.nextField()) {
    if (reader.isEndGroup()) {
      break;
    }
    var field = reader.getFieldNumber();
    switch (field) {
    case 1:
      var value = new proto.aptos.transaction.testing1.v1.WriteSet;
      reader.readMessage(value,proto.aptos.transaction.testing1.v1.WriteSet.deserializeBinaryFromReader);
      msg.setWriteSet(value);
      break;
    default:
      reader.skipField();
      break;
    }
  }
  return msg;
};


/**
 * Serializes the message to binary data (in protobuf wire format).
 * @return {!Uint8Array}
 */
proto.aptos.transaction.testing1.v1.WriteSetPayload.prototype.serializeBinary = function() {
  var writer = new jspb.BinaryWriter();
  proto.aptos.transaction.testing1.v1.WriteSetPayload.serializeBinaryToWriter(this, writer);
  return writer.getResultBuffer();
};


/**
 * Serializes the given message to binary data (in protobuf wire
 * format), writing to the given BinaryWriter.
 * @param {!proto.aptos.transaction.testing1.v1.WriteSetPayload} message
 * @param {!jspb.BinaryWriter} writer
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.aptos.transaction.testing1.v1.WriteSetPayload.serializeBinaryToWriter = function(message, writer) {
  var f = undefined;
  f = message.getWriteSet();
  if (f != null) {
    writer.writeMessage(
      1,
      f,
      proto.aptos.transaction.testing1.v1.WriteSet.serializeBinaryToWriter
    );
  }
};


/**
 * optional WriteSet write_set = 1;
 * @return {?proto.aptos.transaction.testing1.v1.WriteSet}
 */
proto.aptos.transaction.testing1.v1.WriteSetPayload.prototype.getWriteSet = function() {
  return /** @type{?proto.aptos.transaction.testing1.v1.WriteSet} */ (
    jspb.Message.getWrapperField(this, proto.aptos.transaction.testing1.v1.WriteSet, 1));
};


/**
 * @param {?proto.aptos.transaction.testing1.v1.WriteSet|undefined} value
 * @return {!proto.aptos.transaction.testing1.v1.WriteSetPayload} returns this
*/
proto.aptos.transaction.testing1.v1.WriteSetPayload.prototype.setWriteSet = function(value) {
  return jspb.Message.setWrapperField(this, 1, value);
};


/**
 * Clears the message field making it undefined.
 * @return {!proto.aptos.transaction.testing1.v1.WriteSetPayload} returns this
 */
proto.aptos.transaction.testing1.v1.WriteSetPayload.prototype.clearWriteSet = function() {
  return this.setWriteSet(undefined);
};


/**
 * Returns whether this field is set.
 * @return {boolean}
 */
proto.aptos.transaction.testing1.v1.WriteSetPayload.prototype.hasWriteSet = function() {
  return jspb.Message.getField(this, 1) != null;
};





if (jspb.Message.GENERATE_TO_OBJECT) {
/**
 * Creates an object representation of this proto.
 * Field names that are reserved in JavaScript and will be renamed to pb_name.
 * Optional fields that are not set will be set to undefined.
 * To access a reserved field use, foo.pb_<name>, eg, foo.pb_default.
 * For the list of reserved names please see:
 *     net/proto2/compiler/js/internal/generator.cc#kKeyword.
 * @param {boolean=} opt_includeInstance Deprecated. whether to include the
 *     JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @return {!Object}
 */
proto.aptos.transaction.testing1.v1.EntryFunctionId.prototype.toObject = function(opt_includeInstance) {
  return proto.aptos.transaction.testing1.v1.EntryFunctionId.toObject(opt_includeInstance, this);
};


/**
 * Static version of the {@see toObject} method.
 * @param {boolean|undefined} includeInstance Deprecated. Whether to include
 *     the JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @param {!proto.aptos.transaction.testing1.v1.EntryFunctionId} msg The msg instance to transform.
 * @return {!Object}
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.aptos.transaction.testing1.v1.EntryFunctionId.toObject = function(includeInstance, msg) {
  var f, obj = {
    module: (f = msg.getModule()) && proto.aptos.transaction.testing1.v1.MoveModuleId.toObject(includeInstance, f),
    name: jspb.Message.getFieldWithDefault(msg, 2, "")
  };

  if (includeInstance) {
    obj.$jspbMessageInstance = msg;
  }
  return obj;
};
}


/**
 * Deserializes binary data (in protobuf wire format).
 * @param {jspb.ByteSource} bytes The bytes to deserialize.
 * @return {!proto.aptos.transaction.testing1.v1.EntryFunctionId}
 */
proto.aptos.transaction.testing1.v1.EntryFunctionId.deserializeBinary = function(bytes) {
  var reader = new jspb.BinaryReader(bytes);
  var msg = new proto.aptos.transaction.testing1.v1.EntryFunctionId;
  return proto.aptos.transaction.testing1.v1.EntryFunctionId.deserializeBinaryFromReader(msg, reader);
};


/**
 * Deserializes binary data (in protobuf wire format) from the
 * given reader into the given message object.
 * @param {!proto.aptos.transaction.testing1.v1.EntryFunctionId} msg The message object to deserialize into.
 * @param {!jspb.BinaryReader} reader The BinaryReader to use.
 * @return {!proto.aptos.transaction.testing1.v1.EntryFunctionId}
 */
proto.aptos.transaction.testing1.v1.EntryFunctionId.deserializeBinaryFromReader = function(msg, reader) {
  while (reader.nextField()) {
    if (reader.isEndGroup()) {
      break;
    }
    var field = reader.getFieldNumber();
    switch (field) {
    case 1:
      var value = new proto.aptos.transaction.testing1.v1.MoveModuleId;
      reader.readMessage(value,proto.aptos.transaction.testing1.v1.MoveModuleId.deserializeBinaryFromReader);
      msg.setModule(value);
      break;
    case 2:
      var value = /** @type {string} */ (reader.readString());
      msg.setName(value);
      break;
    default:
      reader.skipField();
      break;
    }
  }
  return msg;
};


/**
 * Serializes the message to binary data (in protobuf wire format).
 * @return {!Uint8Array}
 */
proto.aptos.transaction.testing1.v1.EntryFunctionId.prototype.serializeBinary = function() {
  var writer = new jspb.BinaryWriter();
  proto.aptos.transaction.testing1.v1.EntryFunctionId.serializeBinaryToWriter(this, writer);
  return writer.getResultBuffer();
};


/**
 * Serializes the given message to binary data (in protobuf wire
 * format), writing to the given BinaryWriter.
 * @param {!proto.aptos.transaction.testing1.v1.EntryFunctionId} message
 * @param {!jspb.BinaryWriter} writer
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.aptos.transaction.testing1.v1.EntryFunctionId.serializeBinaryToWriter = function(message, writer) {
  var f = undefined;
  f = message.getModule();
  if (f != null) {
    writer.writeMessage(
      1,
      f,
      proto.aptos.transaction.testing1.v1.MoveModuleId.serializeBinaryToWriter
    );
  }
  f = message.getName();
  if (f.length > 0) {
    writer.writeString(
      2,
      f
    );
  }
};


/**
 * optional MoveModuleId module = 1;
 * @return {?proto.aptos.transaction.testing1.v1.MoveModuleId}
 */
proto.aptos.transaction.testing1.v1.EntryFunctionId.prototype.getModule = function() {
  return /** @type{?proto.aptos.transaction.testing1.v1.MoveModuleId} */ (
    jspb.Message.getWrapperField(this, proto.aptos.transaction.testing1.v1.MoveModuleId, 1));
};


/**
 * @param {?proto.aptos.transaction.testing1.v1.MoveModuleId|undefined} value
 * @return {!proto.aptos.transaction.testing1.v1.EntryFunctionId} returns this
*/
proto.aptos.transaction.testing1.v1.EntryFunctionId.prototype.setModule = function(value) {
  return jspb.Message.setWrapperField(this, 1, value);
};


/**
 * Clears the message field making it undefined.
 * @return {!proto.aptos.transaction.testing1.v1.EntryFunctionId} returns this
 */
proto.aptos.transaction.testing1.v1.EntryFunctionId.prototype.clearModule = function() {
  return this.setModule(undefined);
};


/**
 * Returns whether this field is set.
 * @return {boolean}
 */
proto.aptos.transaction.testing1.v1.EntryFunctionId.prototype.hasModule = function() {
  return jspb.Message.getField(this, 1) != null;
};


/**
 * optional string name = 2;
 * @return {string}
 */
proto.aptos.transaction.testing1.v1.EntryFunctionId.prototype.getName = function() {
  return /** @type {string} */ (jspb.Message.getFieldWithDefault(this, 2, ""));
};


/**
 * @param {string} value
 * @return {!proto.aptos.transaction.testing1.v1.EntryFunctionId} returns this
 */
proto.aptos.transaction.testing1.v1.EntryFunctionId.prototype.setName = function(value) {
  return jspb.Message.setProto3StringField(this, 2, value);
};





if (jspb.Message.GENERATE_TO_OBJECT) {
/**
 * Creates an object representation of this proto.
 * Field names that are reserved in JavaScript and will be renamed to pb_name.
 * Optional fields that are not set will be set to undefined.
 * To access a reserved field use, foo.pb_<name>, eg, foo.pb_default.
 * For the list of reserved names please see:
 *     net/proto2/compiler/js/internal/generator.cc#kKeyword.
 * @param {boolean=} opt_includeInstance Deprecated. whether to include the
 *     JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @return {!Object}
 */
proto.aptos.transaction.testing1.v1.MoveModuleId.prototype.toObject = function(opt_includeInstance) {
  return proto.aptos.transaction.testing1.v1.MoveModuleId.toObject(opt_includeInstance, this);
};


/**
 * Static version of the {@see toObject} method.
 * @param {boolean|undefined} includeInstance Deprecated. Whether to include
 *     the JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @param {!proto.aptos.transaction.testing1.v1.MoveModuleId} msg The msg instance to transform.
 * @return {!Object}
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.aptos.transaction.testing1.v1.MoveModuleId.toObject = function(includeInstance, msg) {
  var f, obj = {
    address: jspb.Message.getFieldWithDefault(msg, 1, ""),
    name: jspb.Message.getFieldWithDefault(msg, 2, "")
  };

  if (includeInstance) {
    obj.$jspbMessageInstance = msg;
  }
  return obj;
};
}


/**
 * Deserializes binary data (in protobuf wire format).
 * @param {jspb.ByteSource} bytes The bytes to deserialize.
 * @return {!proto.aptos.transaction.testing1.v1.MoveModuleId}
 */
proto.aptos.transaction.testing1.v1.MoveModuleId.deserializeBinary = function(bytes) {
  var reader = new jspb.BinaryReader(bytes);
  var msg = new proto.aptos.transaction.testing1.v1.MoveModuleId;
  return proto.aptos.transaction.testing1.v1.MoveModuleId.deserializeBinaryFromReader(msg, reader);
};


/**
 * Deserializes binary data (in protobuf wire format) from the
 * given reader into the given message object.
 * @param {!proto.aptos.transaction.testing1.v1.MoveModuleId} msg The message object to deserialize into.
 * @param {!jspb.BinaryReader} reader The BinaryReader to use.
 * @return {!proto.aptos.transaction.testing1.v1.MoveModuleId}
 */
proto.aptos.transaction.testing1.v1.MoveModuleId.deserializeBinaryFromReader = function(msg, reader) {
  while (reader.nextField()) {
    if (reader.isEndGroup()) {
      break;
    }
    var field = reader.getFieldNumber();
    switch (field) {
    case 1:
      var value = /** @type {string} */ (reader.readString());
      msg.setAddress(value);
      break;
    case 2:
      var value = /** @type {string} */ (reader.readString());
      msg.setName(value);
      break;
    default:
      reader.skipField();
      break;
    }
  }
  return msg;
};


/**
 * Serializes the message to binary data (in protobuf wire format).
 * @return {!Uint8Array}
 */
proto.aptos.transaction.testing1.v1.MoveModuleId.prototype.serializeBinary = function() {
  var writer = new jspb.BinaryWriter();
  proto.aptos.transaction.testing1.v1.MoveModuleId.serializeBinaryToWriter(this, writer);
  return writer.getResultBuffer();
};


/**
 * Serializes the given message to binary data (in protobuf wire
 * format), writing to the given BinaryWriter.
 * @param {!proto.aptos.transaction.testing1.v1.MoveModuleId} message
 * @param {!jspb.BinaryWriter} writer
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.aptos.transaction.testing1.v1.MoveModuleId.serializeBinaryToWriter = function(message, writer) {
  var f = undefined;
  f = message.getAddress();
  if (f.length > 0) {
    writer.writeString(
      1,
      f
    );
  }
  f = message.getName();
  if (f.length > 0) {
    writer.writeString(
      2,
      f
    );
  }
};


/**
 * optional string address = 1;
 * @return {string}
 */
proto.aptos.transaction.testing1.v1.MoveModuleId.prototype.getAddress = function() {
  return /** @type {string} */ (jspb.Message.getFieldWithDefault(this, 1, ""));
};


/**
 * @param {string} value
 * @return {!proto.aptos.transaction.testing1.v1.MoveModuleId} returns this
 */
proto.aptos.transaction.testing1.v1.MoveModuleId.prototype.setAddress = function(value) {
  return jspb.Message.setProto3StringField(this, 1, value);
};


/**
 * optional string name = 2;
 * @return {string}
 */
proto.aptos.transaction.testing1.v1.MoveModuleId.prototype.getName = function() {
  return /** @type {string} */ (jspb.Message.getFieldWithDefault(this, 2, ""));
};


/**
 * @param {string} value
 * @return {!proto.aptos.transaction.testing1.v1.MoveModuleId} returns this
 */
proto.aptos.transaction.testing1.v1.MoveModuleId.prototype.setName = function(value) {
  return jspb.Message.setProto3StringField(this, 2, value);
};



/**
 * List of repeated fields within this message type.
 * @private {!Array<number>}
 * @const
 */
proto.aptos.transaction.testing1.v1.MoveStructTag.repeatedFields_ = [4];



if (jspb.Message.GENERATE_TO_OBJECT) {
/**
 * Creates an object representation of this proto.
 * Field names that are reserved in JavaScript and will be renamed to pb_name.
 * Optional fields that are not set will be set to undefined.
 * To access a reserved field use, foo.pb_<name>, eg, foo.pb_default.
 * For the list of reserved names please see:
 *     net/proto2/compiler/js/internal/generator.cc#kKeyword.
 * @param {boolean=} opt_includeInstance Deprecated. whether to include the
 *     JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @return {!Object}
 */
proto.aptos.transaction.testing1.v1.MoveStructTag.prototype.toObject = function(opt_includeInstance) {
  return proto.aptos.transaction.testing1.v1.MoveStructTag.toObject(opt_includeInstance, this);
};


/**
 * Static version of the {@see toObject} method.
 * @param {boolean|undefined} includeInstance Deprecated. Whether to include
 *     the JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @param {!proto.aptos.transaction.testing1.v1.MoveStructTag} msg The msg instance to transform.
 * @return {!Object}
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.aptos.transaction.testing1.v1.MoveStructTag.toObject = function(includeInstance, msg) {
  var f, obj = {
    address: jspb.Message.getFieldWithDefault(msg, 1, ""),
    module: jspb.Message.getFieldWithDefault(msg, 2, ""),
    name: jspb.Message.getFieldWithDefault(msg, 3, ""),
    genericTypeParamsList: jspb.Message.toObjectList(msg.getGenericTypeParamsList(),
    proto.aptos.transaction.testing1.v1.MoveType.toObject, includeInstance)
  };

  if (includeInstance) {
    obj.$jspbMessageInstance = msg;
  }
  return obj;
};
}


/**
 * Deserializes binary data (in protobuf wire format).
 * @param {jspb.ByteSource} bytes The bytes to deserialize.
 * @return {!proto.aptos.transaction.testing1.v1.MoveStructTag}
 */
proto.aptos.transaction.testing1.v1.MoveStructTag.deserializeBinary = function(bytes) {
  var reader = new jspb.BinaryReader(bytes);
  var msg = new proto.aptos.transaction.testing1.v1.MoveStructTag;
  return proto.aptos.transaction.testing1.v1.MoveStructTag.deserializeBinaryFromReader(msg, reader);
};


/**
 * Deserializes binary data (in protobuf wire format) from the
 * given reader into the given message object.
 * @param {!proto.aptos.transaction.testing1.v1.MoveStructTag} msg The message object to deserialize into.
 * @param {!jspb.BinaryReader} reader The BinaryReader to use.
 * @return {!proto.aptos.transaction.testing1.v1.MoveStructTag}
 */
proto.aptos.transaction.testing1.v1.MoveStructTag.deserializeBinaryFromReader = function(msg, reader) {
  while (reader.nextField()) {
    if (reader.isEndGroup()) {
      break;
    }
    var field = reader.getFieldNumber();
    switch (field) {
    case 1:
      var value = /** @type {string} */ (reader.readString());
      msg.setAddress(value);
      break;
    case 2:
      var value = /** @type {string} */ (reader.readString());
      msg.setModule(value);
      break;
    case 3:
      var value = /** @type {string} */ (reader.readString());
      msg.setName(value);
      break;
    case 4:
      var value = new proto.aptos.transaction.testing1.v1.MoveType;
      reader.readMessage(value,proto.aptos.transaction.testing1.v1.MoveType.deserializeBinaryFromReader);
      msg.addGenericTypeParams(value);
      break;
    default:
      reader.skipField();
      break;
    }
  }
  return msg;
};


/**
 * Serializes the message to binary data (in protobuf wire format).
 * @return {!Uint8Array}
 */
proto.aptos.transaction.testing1.v1.MoveStructTag.prototype.serializeBinary = function() {
  var writer = new jspb.BinaryWriter();
  proto.aptos.transaction.testing1.v1.MoveStructTag.serializeBinaryToWriter(this, writer);
  return writer.getResultBuffer();
};


/**
 * Serializes the given message to binary data (in protobuf wire
 * format), writing to the given BinaryWriter.
 * @param {!proto.aptos.transaction.testing1.v1.MoveStructTag} message
 * @param {!jspb.BinaryWriter} writer
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.aptos.transaction.testing1.v1.MoveStructTag.serializeBinaryToWriter = function(message, writer) {
  var f = undefined;
  f = message.getAddress();
  if (f.length > 0) {
    writer.writeString(
      1,
      f
    );
  }
  f = message.getModule();
  if (f.length > 0) {
    writer.writeString(
      2,
      f
    );
  }
  f = message.getName();
  if (f.length > 0) {
    writer.writeString(
      3,
      f
    );
  }
  f = message.getGenericTypeParamsList();
  if (f.length > 0) {
    writer.writeRepeatedMessage(
      4,
      f,
      proto.aptos.transaction.testing1.v1.MoveType.serializeBinaryToWriter
    );
  }
};


/**
 * optional string address = 1;
 * @return {string}
 */
proto.aptos.transaction.testing1.v1.MoveStructTag.prototype.getAddress = function() {
  return /** @type {string} */ (jspb.Message.getFieldWithDefault(this, 1, ""));
};


/**
 * @param {string} value
 * @return {!proto.aptos.transaction.testing1.v1.MoveStructTag} returns this
 */
proto.aptos.transaction.testing1.v1.MoveStructTag.prototype.setAddress = function(value) {
  return jspb.Message.setProto3StringField(this, 1, value);
};


/**
 * optional string module = 2;
 * @return {string}
 */
proto.aptos.transaction.testing1.v1.MoveStructTag.prototype.getModule = function() {
  return /** @type {string} */ (jspb.Message.getFieldWithDefault(this, 2, ""));
};


/**
 * @param {string} value
 * @return {!proto.aptos.transaction.testing1.v1.MoveStructTag} returns this
 */
proto.aptos.transaction.testing1.v1.MoveStructTag.prototype.setModule = function(value) {
  return jspb.Message.setProto3StringField(this, 2, value);
};


/**
 * optional string name = 3;
 * @return {string}
 */
proto.aptos.transaction.testing1.v1.MoveStructTag.prototype.getName = function() {
  return /** @type {string} */ (jspb.Message.getFieldWithDefault(this, 3, ""));
};


/**
 * @param {string} value
 * @return {!proto.aptos.transaction.testing1.v1.MoveStructTag} returns this
 */
proto.aptos.transaction.testing1.v1.MoveStructTag.prototype.setName = function(value) {
  return jspb.Message.setProto3StringField(this, 3, value);
};


/**
 * repeated MoveType generic_type_params = 4;
 * @return {!Array<!proto.aptos.transaction.testing1.v1.MoveType>}
 */
proto.aptos.transaction.testing1.v1.MoveStructTag.prototype.getGenericTypeParamsList = function() {
  return /** @type{!Array<!proto.aptos.transaction.testing1.v1.MoveType>} */ (
    jspb.Message.getRepeatedWrapperField(this, proto.aptos.transaction.testing1.v1.MoveType, 4));
};


/**
 * @param {!Array<!proto.aptos.transaction.testing1.v1.MoveType>} value
 * @return {!proto.aptos.transaction.testing1.v1.MoveStructTag} returns this
*/
proto.aptos.transaction.testing1.v1.MoveStructTag.prototype.setGenericTypeParamsList = function(value) {
  return jspb.Message.setRepeatedWrapperField(this, 4, value);
};


/**
 * @param {!proto.aptos.transaction.testing1.v1.MoveType=} opt_value
 * @param {number=} opt_index
 * @return {!proto.aptos.transaction.testing1.v1.MoveType}
 */
proto.aptos.transaction.testing1.v1.MoveStructTag.prototype.addGenericTypeParams = function(opt_value, opt_index) {
  return jspb.Message.addToRepeatedWrapperField(this, 4, opt_value, proto.aptos.transaction.testing1.v1.MoveType, opt_index);
};


/**
 * Clears the list making it empty but non-null.
 * @return {!proto.aptos.transaction.testing1.v1.MoveStructTag} returns this
 */
proto.aptos.transaction.testing1.v1.MoveStructTag.prototype.clearGenericTypeParamsList = function() {
  return this.setGenericTypeParamsList([]);
};



/**
 * Oneof group definitions for this message. Each group defines the field
 * numbers belonging to that group. When of these fields' value is set, all
 * other fields in the group are cleared. During deserialization, if multiple
 * fields are encountered for a group, only the last value seen will be kept.
 * @private {!Array<!Array<number>>}
 * @const
 */
proto.aptos.transaction.testing1.v1.Signature.oneofGroups_ = [[2,3,4]];

/**
 * @enum {number}
 */
proto.aptos.transaction.testing1.v1.Signature.SignatureCase = {
  SIGNATURE_NOT_SET: 0,
  ED25519: 2,
  MULTI_ED25519: 3,
  MULTI_AGENT: 4
};

/**
 * @return {proto.aptos.transaction.testing1.v1.Signature.SignatureCase}
 */
proto.aptos.transaction.testing1.v1.Signature.prototype.getSignatureCase = function() {
  return /** @type {proto.aptos.transaction.testing1.v1.Signature.SignatureCase} */(jspb.Message.computeOneofCase(this, proto.aptos.transaction.testing1.v1.Signature.oneofGroups_[0]));
};



if (jspb.Message.GENERATE_TO_OBJECT) {
/**
 * Creates an object representation of this proto.
 * Field names that are reserved in JavaScript and will be renamed to pb_name.
 * Optional fields that are not set will be set to undefined.
 * To access a reserved field use, foo.pb_<name>, eg, foo.pb_default.
 * For the list of reserved names please see:
 *     net/proto2/compiler/js/internal/generator.cc#kKeyword.
 * @param {boolean=} opt_includeInstance Deprecated. whether to include the
 *     JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @return {!Object}
 */
proto.aptos.transaction.testing1.v1.Signature.prototype.toObject = function(opt_includeInstance) {
  return proto.aptos.transaction.testing1.v1.Signature.toObject(opt_includeInstance, this);
};


/**
 * Static version of the {@see toObject} method.
 * @param {boolean|undefined} includeInstance Deprecated. Whether to include
 *     the JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @param {!proto.aptos.transaction.testing1.v1.Signature} msg The msg instance to transform.
 * @return {!Object}
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.aptos.transaction.testing1.v1.Signature.toObject = function(includeInstance, msg) {
  var f, obj = {
    type: jspb.Message.getFieldWithDefault(msg, 1, 0),
    ed25519: (f = msg.getEd25519()) && proto.aptos.transaction.testing1.v1.Ed25519Signature.toObject(includeInstance, f),
    multiEd25519: (f = msg.getMultiEd25519()) && proto.aptos.transaction.testing1.v1.MultiEd25519Signature.toObject(includeInstance, f),
    multiAgent: (f = msg.getMultiAgent()) && proto.aptos.transaction.testing1.v1.MultiAgentSignature.toObject(includeInstance, f)
  };

  if (includeInstance) {
    obj.$jspbMessageInstance = msg;
  }
  return obj;
};
}


/**
 * Deserializes binary data (in protobuf wire format).
 * @param {jspb.ByteSource} bytes The bytes to deserialize.
 * @return {!proto.aptos.transaction.testing1.v1.Signature}
 */
proto.aptos.transaction.testing1.v1.Signature.deserializeBinary = function(bytes) {
  var reader = new jspb.BinaryReader(bytes);
  var msg = new proto.aptos.transaction.testing1.v1.Signature;
  return proto.aptos.transaction.testing1.v1.Signature.deserializeBinaryFromReader(msg, reader);
};


/**
 * Deserializes binary data (in protobuf wire format) from the
 * given reader into the given message object.
 * @param {!proto.aptos.transaction.testing1.v1.Signature} msg The message object to deserialize into.
 * @param {!jspb.BinaryReader} reader The BinaryReader to use.
 * @return {!proto.aptos.transaction.testing1.v1.Signature}
 */
proto.aptos.transaction.testing1.v1.Signature.deserializeBinaryFromReader = function(msg, reader) {
  while (reader.nextField()) {
    if (reader.isEndGroup()) {
      break;
    }
    var field = reader.getFieldNumber();
    switch (field) {
    case 1:
      var value = /** @type {!proto.aptos.transaction.testing1.v1.Signature.Type} */ (reader.readEnum());
      msg.setType(value);
      break;
    case 2:
      var value = new proto.aptos.transaction.testing1.v1.Ed25519Signature;
      reader.readMessage(value,proto.aptos.transaction.testing1.v1.Ed25519Signature.deserializeBinaryFromReader);
      msg.setEd25519(value);
      break;
    case 3:
      var value = new proto.aptos.transaction.testing1.v1.MultiEd25519Signature;
      reader.readMessage(value,proto.aptos.transaction.testing1.v1.MultiEd25519Signature.deserializeBinaryFromReader);
      msg.setMultiEd25519(value);
      break;
    case 4:
      var value = new proto.aptos.transaction.testing1.v1.MultiAgentSignature;
      reader.readMessage(value,proto.aptos.transaction.testing1.v1.MultiAgentSignature.deserializeBinaryFromReader);
      msg.setMultiAgent(value);
      break;
    default:
      reader.skipField();
      break;
    }
  }
  return msg;
};


/**
 * Serializes the message to binary data (in protobuf wire format).
 * @return {!Uint8Array}
 */
proto.aptos.transaction.testing1.v1.Signature.prototype.serializeBinary = function() {
  var writer = new jspb.BinaryWriter();
  proto.aptos.transaction.testing1.v1.Signature.serializeBinaryToWriter(this, writer);
  return writer.getResultBuffer();
};


/**
 * Serializes the given message to binary data (in protobuf wire
 * format), writing to the given BinaryWriter.
 * @param {!proto.aptos.transaction.testing1.v1.Signature} message
 * @param {!jspb.BinaryWriter} writer
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.aptos.transaction.testing1.v1.Signature.serializeBinaryToWriter = function(message, writer) {
  var f = undefined;
  f = message.getType();
  if (f !== 0.0) {
    writer.writeEnum(
      1,
      f
    );
  }
  f = message.getEd25519();
  if (f != null) {
    writer.writeMessage(
      2,
      f,
      proto.aptos.transaction.testing1.v1.Ed25519Signature.serializeBinaryToWriter
    );
  }
  f = message.getMultiEd25519();
  if (f != null) {
    writer.writeMessage(
      3,
      f,
      proto.aptos.transaction.testing1.v1.MultiEd25519Signature.serializeBinaryToWriter
    );
  }
  f = message.getMultiAgent();
  if (f != null) {
    writer.writeMessage(
      4,
      f,
      proto.aptos.transaction.testing1.v1.MultiAgentSignature.serializeBinaryToWriter
    );
  }
};


/**
 * @enum {number}
 */
proto.aptos.transaction.testing1.v1.Signature.Type = {
  TYPE_UNSPECIFIED: 0,
  TYPE_ED25519: 1,
  TYPE_MULTI_ED25519: 2,
  TYPE_MULTI_AGENT: 3
};

/**
 * optional Type type = 1;
 * @return {!proto.aptos.transaction.testing1.v1.Signature.Type}
 */
proto.aptos.transaction.testing1.v1.Signature.prototype.getType = function() {
  return /** @type {!proto.aptos.transaction.testing1.v1.Signature.Type} */ (jspb.Message.getFieldWithDefault(this, 1, 0));
};


/**
 * @param {!proto.aptos.transaction.testing1.v1.Signature.Type} value
 * @return {!proto.aptos.transaction.testing1.v1.Signature} returns this
 */
proto.aptos.transaction.testing1.v1.Signature.prototype.setType = function(value) {
  return jspb.Message.setProto3EnumField(this, 1, value);
};


/**
 * optional Ed25519Signature ed25519 = 2;
 * @return {?proto.aptos.transaction.testing1.v1.Ed25519Signature}
 */
proto.aptos.transaction.testing1.v1.Signature.prototype.getEd25519 = function() {
  return /** @type{?proto.aptos.transaction.testing1.v1.Ed25519Signature} */ (
    jspb.Message.getWrapperField(this, proto.aptos.transaction.testing1.v1.Ed25519Signature, 2));
};


/**
 * @param {?proto.aptos.transaction.testing1.v1.Ed25519Signature|undefined} value
 * @return {!proto.aptos.transaction.testing1.v1.Signature} returns this
*/
proto.aptos.transaction.testing1.v1.Signature.prototype.setEd25519 = function(value) {
  return jspb.Message.setOneofWrapperField(this, 2, proto.aptos.transaction.testing1.v1.Signature.oneofGroups_[0], value);
};


/**
 * Clears the message field making it undefined.
 * @return {!proto.aptos.transaction.testing1.v1.Signature} returns this
 */
proto.aptos.transaction.testing1.v1.Signature.prototype.clearEd25519 = function() {
  return this.setEd25519(undefined);
};


/**
 * Returns whether this field is set.
 * @return {boolean}
 */
proto.aptos.transaction.testing1.v1.Signature.prototype.hasEd25519 = function() {
  return jspb.Message.getField(this, 2) != null;
};


/**
 * optional MultiEd25519Signature multi_ed25519 = 3;
 * @return {?proto.aptos.transaction.testing1.v1.MultiEd25519Signature}
 */
proto.aptos.transaction.testing1.v1.Signature.prototype.getMultiEd25519 = function() {
  return /** @type{?proto.aptos.transaction.testing1.v1.MultiEd25519Signature} */ (
    jspb.Message.getWrapperField(this, proto.aptos.transaction.testing1.v1.MultiEd25519Signature, 3));
};


/**
 * @param {?proto.aptos.transaction.testing1.v1.MultiEd25519Signature|undefined} value
 * @return {!proto.aptos.transaction.testing1.v1.Signature} returns this
*/
proto.aptos.transaction.testing1.v1.Signature.prototype.setMultiEd25519 = function(value) {
  return jspb.Message.setOneofWrapperField(this, 3, proto.aptos.transaction.testing1.v1.Signature.oneofGroups_[0], value);
};


/**
 * Clears the message field making it undefined.
 * @return {!proto.aptos.transaction.testing1.v1.Signature} returns this
 */
proto.aptos.transaction.testing1.v1.Signature.prototype.clearMultiEd25519 = function() {
  return this.setMultiEd25519(undefined);
};


/**
 * Returns whether this field is set.
 * @return {boolean}
 */
proto.aptos.transaction.testing1.v1.Signature.prototype.hasMultiEd25519 = function() {
  return jspb.Message.getField(this, 3) != null;
};


/**
 * optional MultiAgentSignature multi_agent = 4;
 * @return {?proto.aptos.transaction.testing1.v1.MultiAgentSignature}
 */
proto.aptos.transaction.testing1.v1.Signature.prototype.getMultiAgent = function() {
  return /** @type{?proto.aptos.transaction.testing1.v1.MultiAgentSignature} */ (
    jspb.Message.getWrapperField(this, proto.aptos.transaction.testing1.v1.MultiAgentSignature, 4));
};


/**
 * @param {?proto.aptos.transaction.testing1.v1.MultiAgentSignature|undefined} value
 * @return {!proto.aptos.transaction.testing1.v1.Signature} returns this
*/
proto.aptos.transaction.testing1.v1.Signature.prototype.setMultiAgent = function(value) {
  return jspb.Message.setOneofWrapperField(this, 4, proto.aptos.transaction.testing1.v1.Signature.oneofGroups_[0], value);
};


/**
 * Clears the message field making it undefined.
 * @return {!proto.aptos.transaction.testing1.v1.Signature} returns this
 */
proto.aptos.transaction.testing1.v1.Signature.prototype.clearMultiAgent = function() {
  return this.setMultiAgent(undefined);
};


/**
 * Returns whether this field is set.
 * @return {boolean}
 */
proto.aptos.transaction.testing1.v1.Signature.prototype.hasMultiAgent = function() {
  return jspb.Message.getField(this, 4) != null;
};





if (jspb.Message.GENERATE_TO_OBJECT) {
/**
 * Creates an object representation of this proto.
 * Field names that are reserved in JavaScript and will be renamed to pb_name.
 * Optional fields that are not set will be set to undefined.
 * To access a reserved field use, foo.pb_<name>, eg, foo.pb_default.
 * For the list of reserved names please see:
 *     net/proto2/compiler/js/internal/generator.cc#kKeyword.
 * @param {boolean=} opt_includeInstance Deprecated. whether to include the
 *     JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @return {!Object}
 */
proto.aptos.transaction.testing1.v1.Ed25519Signature.prototype.toObject = function(opt_includeInstance) {
  return proto.aptos.transaction.testing1.v1.Ed25519Signature.toObject(opt_includeInstance, this);
};


/**
 * Static version of the {@see toObject} method.
 * @param {boolean|undefined} includeInstance Deprecated. Whether to include
 *     the JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @param {!proto.aptos.transaction.testing1.v1.Ed25519Signature} msg The msg instance to transform.
 * @return {!Object}
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.aptos.transaction.testing1.v1.Ed25519Signature.toObject = function(includeInstance, msg) {
  var f, obj = {
    publicKey: msg.getPublicKey_asB64(),
    signature: msg.getSignature_asB64()
  };

  if (includeInstance) {
    obj.$jspbMessageInstance = msg;
  }
  return obj;
};
}


/**
 * Deserializes binary data (in protobuf wire format).
 * @param {jspb.ByteSource} bytes The bytes to deserialize.
 * @return {!proto.aptos.transaction.testing1.v1.Ed25519Signature}
 */
proto.aptos.transaction.testing1.v1.Ed25519Signature.deserializeBinary = function(bytes) {
  var reader = new jspb.BinaryReader(bytes);
  var msg = new proto.aptos.transaction.testing1.v1.Ed25519Signature;
  return proto.aptos.transaction.testing1.v1.Ed25519Signature.deserializeBinaryFromReader(msg, reader);
};


/**
 * Deserializes binary data (in protobuf wire format) from the
 * given reader into the given message object.
 * @param {!proto.aptos.transaction.testing1.v1.Ed25519Signature} msg The message object to deserialize into.
 * @param {!jspb.BinaryReader} reader The BinaryReader to use.
 * @return {!proto.aptos.transaction.testing1.v1.Ed25519Signature}
 */
proto.aptos.transaction.testing1.v1.Ed25519Signature.deserializeBinaryFromReader = function(msg, reader) {
  while (reader.nextField()) {
    if (reader.isEndGroup()) {
      break;
    }
    var field = reader.getFieldNumber();
    switch (field) {
    case 1:
      var value = /** @type {!Uint8Array} */ (reader.readBytes());
      msg.setPublicKey(value);
      break;
    case 2:
      var value = /** @type {!Uint8Array} */ (reader.readBytes());
      msg.setSignature(value);
      break;
    default:
      reader.skipField();
      break;
    }
  }
  return msg;
};


/**
 * Serializes the message to binary data (in protobuf wire format).
 * @return {!Uint8Array}
 */
proto.aptos.transaction.testing1.v1.Ed25519Signature.prototype.serializeBinary = function() {
  var writer = new jspb.BinaryWriter();
  proto.aptos.transaction.testing1.v1.Ed25519Signature.serializeBinaryToWriter(this, writer);
  return writer.getResultBuffer();
};


/**
 * Serializes the given message to binary data (in protobuf wire
 * format), writing to the given BinaryWriter.
 * @param {!proto.aptos.transaction.testing1.v1.Ed25519Signature} message
 * @param {!jspb.BinaryWriter} writer
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.aptos.transaction.testing1.v1.Ed25519Signature.serializeBinaryToWriter = function(message, writer) {
  var f = undefined;
  f = message.getPublicKey_asU8();
  if (f.length > 0) {
    writer.writeBytes(
      1,
      f
    );
  }
  f = message.getSignature_asU8();
  if (f.length > 0) {
    writer.writeBytes(
      2,
      f
    );
  }
};


/**
 * optional bytes public_key = 1;
 * @return {!(string|Uint8Array)}
 */
proto.aptos.transaction.testing1.v1.Ed25519Signature.prototype.getPublicKey = function() {
  return /** @type {!(string|Uint8Array)} */ (jspb.Message.getFieldWithDefault(this, 1, ""));
};


/**
 * optional bytes public_key = 1;
 * This is a type-conversion wrapper around `getPublicKey()`
 * @return {string}
 */
proto.aptos.transaction.testing1.v1.Ed25519Signature.prototype.getPublicKey_asB64 = function() {
  return /** @type {string} */ (jspb.Message.bytesAsB64(
      this.getPublicKey()));
};


/**
 * optional bytes public_key = 1;
 * Note that Uint8Array is not supported on all browsers.
 * @see http://caniuse.com/Uint8Array
 * This is a type-conversion wrapper around `getPublicKey()`
 * @return {!Uint8Array}
 */
proto.aptos.transaction.testing1.v1.Ed25519Signature.prototype.getPublicKey_asU8 = function() {
  return /** @type {!Uint8Array} */ (jspb.Message.bytesAsU8(
      this.getPublicKey()));
};


/**
 * @param {!(string|Uint8Array)} value
 * @return {!proto.aptos.transaction.testing1.v1.Ed25519Signature} returns this
 */
proto.aptos.transaction.testing1.v1.Ed25519Signature.prototype.setPublicKey = function(value) {
  return jspb.Message.setProto3BytesField(this, 1, value);
};


/**
 * optional bytes signature = 2;
 * @return {!(string|Uint8Array)}
 */
proto.aptos.transaction.testing1.v1.Ed25519Signature.prototype.getSignature = function() {
  return /** @type {!(string|Uint8Array)} */ (jspb.Message.getFieldWithDefault(this, 2, ""));
};


/**
 * optional bytes signature = 2;
 * This is a type-conversion wrapper around `getSignature()`
 * @return {string}
 */
proto.aptos.transaction.testing1.v1.Ed25519Signature.prototype.getSignature_asB64 = function() {
  return /** @type {string} */ (jspb.Message.bytesAsB64(
      this.getSignature()));
};


/**
 * optional bytes signature = 2;
 * Note that Uint8Array is not supported on all browsers.
 * @see http://caniuse.com/Uint8Array
 * This is a type-conversion wrapper around `getSignature()`
 * @return {!Uint8Array}
 */
proto.aptos.transaction.testing1.v1.Ed25519Signature.prototype.getSignature_asU8 = function() {
  return /** @type {!Uint8Array} */ (jspb.Message.bytesAsU8(
      this.getSignature()));
};


/**
 * @param {!(string|Uint8Array)} value
 * @return {!proto.aptos.transaction.testing1.v1.Ed25519Signature} returns this
 */
proto.aptos.transaction.testing1.v1.Ed25519Signature.prototype.setSignature = function(value) {
  return jspb.Message.setProto3BytesField(this, 2, value);
};



/**
 * List of repeated fields within this message type.
 * @private {!Array<number>}
 * @const
 */
proto.aptos.transaction.testing1.v1.MultiEd25519Signature.repeatedFields_ = [1,2,4];



if (jspb.Message.GENERATE_TO_OBJECT) {
/**
 * Creates an object representation of this proto.
 * Field names that are reserved in JavaScript and will be renamed to pb_name.
 * Optional fields that are not set will be set to undefined.
 * To access a reserved field use, foo.pb_<name>, eg, foo.pb_default.
 * For the list of reserved names please see:
 *     net/proto2/compiler/js/internal/generator.cc#kKeyword.
 * @param {boolean=} opt_includeInstance Deprecated. whether to include the
 *     JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @return {!Object}
 */
proto.aptos.transaction.testing1.v1.MultiEd25519Signature.prototype.toObject = function(opt_includeInstance) {
  return proto.aptos.transaction.testing1.v1.MultiEd25519Signature.toObject(opt_includeInstance, this);
};


/**
 * Static version of the {@see toObject} method.
 * @param {boolean|undefined} includeInstance Deprecated. Whether to include
 *     the JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @param {!proto.aptos.transaction.testing1.v1.MultiEd25519Signature} msg The msg instance to transform.
 * @return {!Object}
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.aptos.transaction.testing1.v1.MultiEd25519Signature.toObject = function(includeInstance, msg) {
  var f, obj = {
    publicKeysList: msg.getPublicKeysList_asB64(),
    signaturesList: msg.getSignaturesList_asB64(),
    threshold: jspb.Message.getFieldWithDefault(msg, 3, 0),
    publicKeyIndicesList: (f = jspb.Message.getRepeatedField(msg, 4)) == null ? undefined : f
  };

  if (includeInstance) {
    obj.$jspbMessageInstance = msg;
  }
  return obj;
};
}


/**
 * Deserializes binary data (in protobuf wire format).
 * @param {jspb.ByteSource} bytes The bytes to deserialize.
 * @return {!proto.aptos.transaction.testing1.v1.MultiEd25519Signature}
 */
proto.aptos.transaction.testing1.v1.MultiEd25519Signature.deserializeBinary = function(bytes) {
  var reader = new jspb.BinaryReader(bytes);
  var msg = new proto.aptos.transaction.testing1.v1.MultiEd25519Signature;
  return proto.aptos.transaction.testing1.v1.MultiEd25519Signature.deserializeBinaryFromReader(msg, reader);
};


/**
 * Deserializes binary data (in protobuf wire format) from the
 * given reader into the given message object.
 * @param {!proto.aptos.transaction.testing1.v1.MultiEd25519Signature} msg The message object to deserialize into.
 * @param {!jspb.BinaryReader} reader The BinaryReader to use.
 * @return {!proto.aptos.transaction.testing1.v1.MultiEd25519Signature}
 */
proto.aptos.transaction.testing1.v1.MultiEd25519Signature.deserializeBinaryFromReader = function(msg, reader) {
  while (reader.nextField()) {
    if (reader.isEndGroup()) {
      break;
    }
    var field = reader.getFieldNumber();
    switch (field) {
    case 1:
      var value = /** @type {!Uint8Array} */ (reader.readBytes());
      msg.addPublicKeys(value);
      break;
    case 2:
      var value = /** @type {!Uint8Array} */ (reader.readBytes());
      msg.addSignatures(value);
      break;
    case 3:
      var value = /** @type {number} */ (reader.readUint32());
      msg.setThreshold(value);
      break;
    case 4:
      var values = /** @type {!Array<number>} */ (reader.isDelimited() ? reader.readPackedUint32() : [reader.readUint32()]);
      for (var i = 0; i < values.length; i++) {
        msg.addPublicKeyIndices(values[i]);
      }
      break;
    default:
      reader.skipField();
      break;
    }
  }
  return msg;
};


/**
 * Serializes the message to binary data (in protobuf wire format).
 * @return {!Uint8Array}
 */
proto.aptos.transaction.testing1.v1.MultiEd25519Signature.prototype.serializeBinary = function() {
  var writer = new jspb.BinaryWriter();
  proto.aptos.transaction.testing1.v1.MultiEd25519Signature.serializeBinaryToWriter(this, writer);
  return writer.getResultBuffer();
};


/**
 * Serializes the given message to binary data (in protobuf wire
 * format), writing to the given BinaryWriter.
 * @param {!proto.aptos.transaction.testing1.v1.MultiEd25519Signature} message
 * @param {!jspb.BinaryWriter} writer
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.aptos.transaction.testing1.v1.MultiEd25519Signature.serializeBinaryToWriter = function(message, writer) {
  var f = undefined;
  f = message.getPublicKeysList_asU8();
  if (f.length > 0) {
    writer.writeRepeatedBytes(
      1,
      f
    );
  }
  f = message.getSignaturesList_asU8();
  if (f.length > 0) {
    writer.writeRepeatedBytes(
      2,
      f
    );
  }
  f = message.getThreshold();
  if (f !== 0) {
    writer.writeUint32(
      3,
      f
    );
  }
  f = message.getPublicKeyIndicesList();
  if (f.length > 0) {
    writer.writePackedUint32(
      4,
      f
    );
  }
};


/**
 * repeated bytes public_keys = 1;
 * @return {!(Array<!Uint8Array>|Array<string>)}
 */
proto.aptos.transaction.testing1.v1.MultiEd25519Signature.prototype.getPublicKeysList = function() {
  return /** @type {!(Array<!Uint8Array>|Array<string>)} */ (jspb.Message.getRepeatedField(this, 1));
};


/**
 * repeated bytes public_keys = 1;
 * This is a type-conversion wrapper around `getPublicKeysList()`
 * @return {!Array<string>}
 */
proto.aptos.transaction.testing1.v1.MultiEd25519Signature.prototype.getPublicKeysList_asB64 = function() {
  return /** @type {!Array<string>} */ (jspb.Message.bytesListAsB64(
      this.getPublicKeysList()));
};


/**
 * repeated bytes public_keys = 1;
 * Note that Uint8Array is not supported on all browsers.
 * @see http://caniuse.com/Uint8Array
 * This is a type-conversion wrapper around `getPublicKeysList()`
 * @return {!Array<!Uint8Array>}
 */
proto.aptos.transaction.testing1.v1.MultiEd25519Signature.prototype.getPublicKeysList_asU8 = function() {
  return /** @type {!Array<!Uint8Array>} */ (jspb.Message.bytesListAsU8(
      this.getPublicKeysList()));
};


/**
 * @param {!(Array<!Uint8Array>|Array<string>)} value
 * @return {!proto.aptos.transaction.testing1.v1.MultiEd25519Signature} returns this
 */
proto.aptos.transaction.testing1.v1.MultiEd25519Signature.prototype.setPublicKeysList = function(value) {
  return jspb.Message.setField(this, 1, value || []);
};


/**
 * @param {!(string|Uint8Array)} value
 * @param {number=} opt_index
 * @return {!proto.aptos.transaction.testing1.v1.MultiEd25519Signature} returns this
 */
proto.aptos.transaction.testing1.v1.MultiEd25519Signature.prototype.addPublicKeys = function(value, opt_index) {
  return jspb.Message.addToRepeatedField(this, 1, value, opt_index);
};


/**
 * Clears the list making it empty but non-null.
 * @return {!proto.aptos.transaction.testing1.v1.MultiEd25519Signature} returns this
 */
proto.aptos.transaction.testing1.v1.MultiEd25519Signature.prototype.clearPublicKeysList = function() {
  return this.setPublicKeysList([]);
};


/**
 * repeated bytes signatures = 2;
 * @return {!(Array<!Uint8Array>|Array<string>)}
 */
proto.aptos.transaction.testing1.v1.MultiEd25519Signature.prototype.getSignaturesList = function() {
  return /** @type {!(Array<!Uint8Array>|Array<string>)} */ (jspb.Message.getRepeatedField(this, 2));
};


/**
 * repeated bytes signatures = 2;
 * This is a type-conversion wrapper around `getSignaturesList()`
 * @return {!Array<string>}
 */
proto.aptos.transaction.testing1.v1.MultiEd25519Signature.prototype.getSignaturesList_asB64 = function() {
  return /** @type {!Array<string>} */ (jspb.Message.bytesListAsB64(
      this.getSignaturesList()));
};


/**
 * repeated bytes signatures = 2;
 * Note that Uint8Array is not supported on all browsers.
 * @see http://caniuse.com/Uint8Array
 * This is a type-conversion wrapper around `getSignaturesList()`
 * @return {!Array<!Uint8Array>}
 */
proto.aptos.transaction.testing1.v1.MultiEd25519Signature.prototype.getSignaturesList_asU8 = function() {
  return /** @type {!Array<!Uint8Array>} */ (jspb.Message.bytesListAsU8(
      this.getSignaturesList()));
};


/**
 * @param {!(Array<!Uint8Array>|Array<string>)} value
 * @return {!proto.aptos.transaction.testing1.v1.MultiEd25519Signature} returns this
 */
proto.aptos.transaction.testing1.v1.MultiEd25519Signature.prototype.setSignaturesList = function(value) {
  return jspb.Message.setField(this, 2, value || []);
};


/**
 * @param {!(string|Uint8Array)} value
 * @param {number=} opt_index
 * @return {!proto.aptos.transaction.testing1.v1.MultiEd25519Signature} returns this
 */
proto.aptos.transaction.testing1.v1.MultiEd25519Signature.prototype.addSignatures = function(value, opt_index) {
  return jspb.Message.addToRepeatedField(this, 2, value, opt_index);
};


/**
 * Clears the list making it empty but non-null.
 * @return {!proto.aptos.transaction.testing1.v1.MultiEd25519Signature} returns this
 */
proto.aptos.transaction.testing1.v1.MultiEd25519Signature.prototype.clearSignaturesList = function() {
  return this.setSignaturesList([]);
};


/**
 * optional uint32 threshold = 3;
 * @return {number}
 */
proto.aptos.transaction.testing1.v1.MultiEd25519Signature.prototype.getThreshold = function() {
  return /** @type {number} */ (jspb.Message.getFieldWithDefault(this, 3, 0));
};


/**
 * @param {number} value
 * @return {!proto.aptos.transaction.testing1.v1.MultiEd25519Signature} returns this
 */
proto.aptos.transaction.testing1.v1.MultiEd25519Signature.prototype.setThreshold = function(value) {
  return jspb.Message.setProto3IntField(this, 3, value);
};


/**
 * repeated uint32 public_key_indices = 4;
 * @return {!Array<number>}
 */
proto.aptos.transaction.testing1.v1.MultiEd25519Signature.prototype.getPublicKeyIndicesList = function() {
  return /** @type {!Array<number>} */ (jspb.Message.getRepeatedField(this, 4));
};


/**
 * @param {!Array<number>} value
 * @return {!proto.aptos.transaction.testing1.v1.MultiEd25519Signature} returns this
 */
proto.aptos.transaction.testing1.v1.MultiEd25519Signature.prototype.setPublicKeyIndicesList = function(value) {
  return jspb.Message.setField(this, 4, value || []);
};


/**
 * @param {number} value
 * @param {number=} opt_index
 * @return {!proto.aptos.transaction.testing1.v1.MultiEd25519Signature} returns this
 */
proto.aptos.transaction.testing1.v1.MultiEd25519Signature.prototype.addPublicKeyIndices = function(value, opt_index) {
  return jspb.Message.addToRepeatedField(this, 4, value, opt_index);
};


/**
 * Clears the list making it empty but non-null.
 * @return {!proto.aptos.transaction.testing1.v1.MultiEd25519Signature} returns this
 */
proto.aptos.transaction.testing1.v1.MultiEd25519Signature.prototype.clearPublicKeyIndicesList = function() {
  return this.setPublicKeyIndicesList([]);
};



/**
 * List of repeated fields within this message type.
 * @private {!Array<number>}
 * @const
 */
proto.aptos.transaction.testing1.v1.MultiAgentSignature.repeatedFields_ = [2,3];



if (jspb.Message.GENERATE_TO_OBJECT) {
/**
 * Creates an object representation of this proto.
 * Field names that are reserved in JavaScript and will be renamed to pb_name.
 * Optional fields that are not set will be set to undefined.
 * To access a reserved field use, foo.pb_<name>, eg, foo.pb_default.
 * For the list of reserved names please see:
 *     net/proto2/compiler/js/internal/generator.cc#kKeyword.
 * @param {boolean=} opt_includeInstance Deprecated. whether to include the
 *     JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @return {!Object}
 */
proto.aptos.transaction.testing1.v1.MultiAgentSignature.prototype.toObject = function(opt_includeInstance) {
  return proto.aptos.transaction.testing1.v1.MultiAgentSignature.toObject(opt_includeInstance, this);
};


/**
 * Static version of the {@see toObject} method.
 * @param {boolean|undefined} includeInstance Deprecated. Whether to include
 *     the JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @param {!proto.aptos.transaction.testing1.v1.MultiAgentSignature} msg The msg instance to transform.
 * @return {!Object}
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.aptos.transaction.testing1.v1.MultiAgentSignature.toObject = function(includeInstance, msg) {
  var f, obj = {
    sender: (f = msg.getSender()) && proto.aptos.transaction.testing1.v1.AccountSignature.toObject(includeInstance, f),
    secondarySignerAddressesList: (f = jspb.Message.getRepeatedField(msg, 2)) == null ? undefined : f,
    secondarySignersList: jspb.Message.toObjectList(msg.getSecondarySignersList(),
    proto.aptos.transaction.testing1.v1.AccountSignature.toObject, includeInstance)
  };

  if (includeInstance) {
    obj.$jspbMessageInstance = msg;
  }
  return obj;
};
}


/**
 * Deserializes binary data (in protobuf wire format).
 * @param {jspb.ByteSource} bytes The bytes to deserialize.
 * @return {!proto.aptos.transaction.testing1.v1.MultiAgentSignature}
 */
proto.aptos.transaction.testing1.v1.MultiAgentSignature.deserializeBinary = function(bytes) {
  var reader = new jspb.BinaryReader(bytes);
  var msg = new proto.aptos.transaction.testing1.v1.MultiAgentSignature;
  return proto.aptos.transaction.testing1.v1.MultiAgentSignature.deserializeBinaryFromReader(msg, reader);
};


/**
 * Deserializes binary data (in protobuf wire format) from the
 * given reader into the given message object.
 * @param {!proto.aptos.transaction.testing1.v1.MultiAgentSignature} msg The message object to deserialize into.
 * @param {!jspb.BinaryReader} reader The BinaryReader to use.
 * @return {!proto.aptos.transaction.testing1.v1.MultiAgentSignature}
 */
proto.aptos.transaction.testing1.v1.MultiAgentSignature.deserializeBinaryFromReader = function(msg, reader) {
  while (reader.nextField()) {
    if (reader.isEndGroup()) {
      break;
    }
    var field = reader.getFieldNumber();
    switch (field) {
    case 1:
      var value = new proto.aptos.transaction.testing1.v1.AccountSignature;
      reader.readMessage(value,proto.aptos.transaction.testing1.v1.AccountSignature.deserializeBinaryFromReader);
      msg.setSender(value);
      break;
    case 2:
      var value = /** @type {string} */ (reader.readString());
      msg.addSecondarySignerAddresses(value);
      break;
    case 3:
      var value = new proto.aptos.transaction.testing1.v1.AccountSignature;
      reader.readMessage(value,proto.aptos.transaction.testing1.v1.AccountSignature.deserializeBinaryFromReader);
      msg.addSecondarySigners(value);
      break;
    default:
      reader.skipField();
      break;
    }
  }
  return msg;
};


/**
 * Serializes the message to binary data (in protobuf wire format).
 * @return {!Uint8Array}
 */
proto.aptos.transaction.testing1.v1.MultiAgentSignature.prototype.serializeBinary = function() {
  var writer = new jspb.BinaryWriter();
  proto.aptos.transaction.testing1.v1.MultiAgentSignature.serializeBinaryToWriter(this, writer);
  return writer.getResultBuffer();
};


/**
 * Serializes the given message to binary data (in protobuf wire
 * format), writing to the given BinaryWriter.
 * @param {!proto.aptos.transaction.testing1.v1.MultiAgentSignature} message
 * @param {!jspb.BinaryWriter} writer
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.aptos.transaction.testing1.v1.MultiAgentSignature.serializeBinaryToWriter = function(message, writer) {
  var f = undefined;
  f = message.getSender();
  if (f != null) {
    writer.writeMessage(
      1,
      f,
      proto.aptos.transaction.testing1.v1.AccountSignature.serializeBinaryToWriter
    );
  }
  f = message.getSecondarySignerAddressesList();
  if (f.length > 0) {
    writer.writeRepeatedString(
      2,
      f
    );
  }
  f = message.getSecondarySignersList();
  if (f.length > 0) {
    writer.writeRepeatedMessage(
      3,
      f,
      proto.aptos.transaction.testing1.v1.AccountSignature.serializeBinaryToWriter
    );
  }
};


/**
 * optional AccountSignature sender = 1;
 * @return {?proto.aptos.transaction.testing1.v1.AccountSignature}
 */
proto.aptos.transaction.testing1.v1.MultiAgentSignature.prototype.getSender = function() {
  return /** @type{?proto.aptos.transaction.testing1.v1.AccountSignature} */ (
    jspb.Message.getWrapperField(this, proto.aptos.transaction.testing1.v1.AccountSignature, 1));
};


/**
 * @param {?proto.aptos.transaction.testing1.v1.AccountSignature|undefined} value
 * @return {!proto.aptos.transaction.testing1.v1.MultiAgentSignature} returns this
*/
proto.aptos.transaction.testing1.v1.MultiAgentSignature.prototype.setSender = function(value) {
  return jspb.Message.setWrapperField(this, 1, value);
};


/**
 * Clears the message field making it undefined.
 * @return {!proto.aptos.transaction.testing1.v1.MultiAgentSignature} returns this
 */
proto.aptos.transaction.testing1.v1.MultiAgentSignature.prototype.clearSender = function() {
  return this.setSender(undefined);
};


/**
 * Returns whether this field is set.
 * @return {boolean}
 */
proto.aptos.transaction.testing1.v1.MultiAgentSignature.prototype.hasSender = function() {
  return jspb.Message.getField(this, 1) != null;
};


/**
 * repeated string secondary_signer_addresses = 2;
 * @return {!Array<string>}
 */
proto.aptos.transaction.testing1.v1.MultiAgentSignature.prototype.getSecondarySignerAddressesList = function() {
  return /** @type {!Array<string>} */ (jspb.Message.getRepeatedField(this, 2));
};


/**
 * @param {!Array<string>} value
 * @return {!proto.aptos.transaction.testing1.v1.MultiAgentSignature} returns this
 */
proto.aptos.transaction.testing1.v1.MultiAgentSignature.prototype.setSecondarySignerAddressesList = function(value) {
  return jspb.Message.setField(this, 2, value || []);
};


/**
 * @param {string} value
 * @param {number=} opt_index
 * @return {!proto.aptos.transaction.testing1.v1.MultiAgentSignature} returns this
 */
proto.aptos.transaction.testing1.v1.MultiAgentSignature.prototype.addSecondarySignerAddresses = function(value, opt_index) {
  return jspb.Message.addToRepeatedField(this, 2, value, opt_index);
};


/**
 * Clears the list making it empty but non-null.
 * @return {!proto.aptos.transaction.testing1.v1.MultiAgentSignature} returns this
 */
proto.aptos.transaction.testing1.v1.MultiAgentSignature.prototype.clearSecondarySignerAddressesList = function() {
  return this.setSecondarySignerAddressesList([]);
};


/**
 * repeated AccountSignature secondary_signers = 3;
 * @return {!Array<!proto.aptos.transaction.testing1.v1.AccountSignature>}
 */
proto.aptos.transaction.testing1.v1.MultiAgentSignature.prototype.getSecondarySignersList = function() {
  return /** @type{!Array<!proto.aptos.transaction.testing1.v1.AccountSignature>} */ (
    jspb.Message.getRepeatedWrapperField(this, proto.aptos.transaction.testing1.v1.AccountSignature, 3));
};


/**
 * @param {!Array<!proto.aptos.transaction.testing1.v1.AccountSignature>} value
 * @return {!proto.aptos.transaction.testing1.v1.MultiAgentSignature} returns this
*/
proto.aptos.transaction.testing1.v1.MultiAgentSignature.prototype.setSecondarySignersList = function(value) {
  return jspb.Message.setRepeatedWrapperField(this, 3, value);
};


/**
 * @param {!proto.aptos.transaction.testing1.v1.AccountSignature=} opt_value
 * @param {number=} opt_index
 * @return {!proto.aptos.transaction.testing1.v1.AccountSignature}
 */
proto.aptos.transaction.testing1.v1.MultiAgentSignature.prototype.addSecondarySigners = function(opt_value, opt_index) {
  return jspb.Message.addToRepeatedWrapperField(this, 3, opt_value, proto.aptos.transaction.testing1.v1.AccountSignature, opt_index);
};


/**
 * Clears the list making it empty but non-null.
 * @return {!proto.aptos.transaction.testing1.v1.MultiAgentSignature} returns this
 */
proto.aptos.transaction.testing1.v1.MultiAgentSignature.prototype.clearSecondarySignersList = function() {
  return this.setSecondarySignersList([]);
};



/**
 * Oneof group definitions for this message. Each group defines the field
 * numbers belonging to that group. When of these fields' value is set, all
 * other fields in the group are cleared. During deserialization, if multiple
 * fields are encountered for a group, only the last value seen will be kept.
 * @private {!Array<!Array<number>>}
 * @const
 */
proto.aptos.transaction.testing1.v1.AccountSignature.oneofGroups_ = [[2,3]];

/**
 * @enum {number}
 */
proto.aptos.transaction.testing1.v1.AccountSignature.SignatureCase = {
  SIGNATURE_NOT_SET: 0,
  ED25519: 2,
  MULTI_ED25519: 3
};

/**
 * @return {proto.aptos.transaction.testing1.v1.AccountSignature.SignatureCase}
 */
proto.aptos.transaction.testing1.v1.AccountSignature.prototype.getSignatureCase = function() {
  return /** @type {proto.aptos.transaction.testing1.v1.AccountSignature.SignatureCase} */(jspb.Message.computeOneofCase(this, proto.aptos.transaction.testing1.v1.AccountSignature.oneofGroups_[0]));
};



if (jspb.Message.GENERATE_TO_OBJECT) {
/**
 * Creates an object representation of this proto.
 * Field names that are reserved in JavaScript and will be renamed to pb_name.
 * Optional fields that are not set will be set to undefined.
 * To access a reserved field use, foo.pb_<name>, eg, foo.pb_default.
 * For the list of reserved names please see:
 *     net/proto2/compiler/js/internal/generator.cc#kKeyword.
 * @param {boolean=} opt_includeInstance Deprecated. whether to include the
 *     JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @return {!Object}
 */
proto.aptos.transaction.testing1.v1.AccountSignature.prototype.toObject = function(opt_includeInstance) {
  return proto.aptos.transaction.testing1.v1.AccountSignature.toObject(opt_includeInstance, this);
};


/**
 * Static version of the {@see toObject} method.
 * @param {boolean|undefined} includeInstance Deprecated. Whether to include
 *     the JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @param {!proto.aptos.transaction.testing1.v1.AccountSignature} msg The msg instance to transform.
 * @return {!Object}
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.aptos.transaction.testing1.v1.AccountSignature.toObject = function(includeInstance, msg) {
  var f, obj = {
    type: jspb.Message.getFieldWithDefault(msg, 1, 0),
    ed25519: (f = msg.getEd25519()) && proto.aptos.transaction.testing1.v1.Ed25519Signature.toObject(includeInstance, f),
    multiEd25519: (f = msg.getMultiEd25519()) && proto.aptos.transaction.testing1.v1.MultiEd25519Signature.toObject(includeInstance, f)
  };

  if (includeInstance) {
    obj.$jspbMessageInstance = msg;
  }
  return obj;
};
}


/**
 * Deserializes binary data (in protobuf wire format).
 * @param {jspb.ByteSource} bytes The bytes to deserialize.
 * @return {!proto.aptos.transaction.testing1.v1.AccountSignature}
 */
proto.aptos.transaction.testing1.v1.AccountSignature.deserializeBinary = function(bytes) {
  var reader = new jspb.BinaryReader(bytes);
  var msg = new proto.aptos.transaction.testing1.v1.AccountSignature;
  return proto.aptos.transaction.testing1.v1.AccountSignature.deserializeBinaryFromReader(msg, reader);
};


/**
 * Deserializes binary data (in protobuf wire format) from the
 * given reader into the given message object.
 * @param {!proto.aptos.transaction.testing1.v1.AccountSignature} msg The message object to deserialize into.
 * @param {!jspb.BinaryReader} reader The BinaryReader to use.
 * @return {!proto.aptos.transaction.testing1.v1.AccountSignature}
 */
proto.aptos.transaction.testing1.v1.AccountSignature.deserializeBinaryFromReader = function(msg, reader) {
  while (reader.nextField()) {
    if (reader.isEndGroup()) {
      break;
    }
    var field = reader.getFieldNumber();
    switch (field) {
    case 1:
      var value = /** @type {!proto.aptos.transaction.testing1.v1.AccountSignature.Type} */ (reader.readEnum());
      msg.setType(value);
      break;
    case 2:
      var value = new proto.aptos.transaction.testing1.v1.Ed25519Signature;
      reader.readMessage(value,proto.aptos.transaction.testing1.v1.Ed25519Signature.deserializeBinaryFromReader);
      msg.setEd25519(value);
      break;
    case 3:
      var value = new proto.aptos.transaction.testing1.v1.MultiEd25519Signature;
      reader.readMessage(value,proto.aptos.transaction.testing1.v1.MultiEd25519Signature.deserializeBinaryFromReader);
      msg.setMultiEd25519(value);
      break;
    default:
      reader.skipField();
      break;
    }
  }
  return msg;
};


/**
 * Serializes the message to binary data (in protobuf wire format).
 * @return {!Uint8Array}
 */
proto.aptos.transaction.testing1.v1.AccountSignature.prototype.serializeBinary = function() {
  var writer = new jspb.BinaryWriter();
  proto.aptos.transaction.testing1.v1.AccountSignature.serializeBinaryToWriter(this, writer);
  return writer.getResultBuffer();
};


/**
 * Serializes the given message to binary data (in protobuf wire
 * format), writing to the given BinaryWriter.
 * @param {!proto.aptos.transaction.testing1.v1.AccountSignature} message
 * @param {!jspb.BinaryWriter} writer
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.aptos.transaction.testing1.v1.AccountSignature.serializeBinaryToWriter = function(message, writer) {
  var f = undefined;
  f = message.getType();
  if (f !== 0.0) {
    writer.writeEnum(
      1,
      f
    );
  }
  f = message.getEd25519();
  if (f != null) {
    writer.writeMessage(
      2,
      f,
      proto.aptos.transaction.testing1.v1.Ed25519Signature.serializeBinaryToWriter
    );
  }
  f = message.getMultiEd25519();
  if (f != null) {
    writer.writeMessage(
      3,
      f,
      proto.aptos.transaction.testing1.v1.MultiEd25519Signature.serializeBinaryToWriter
    );
  }
};


/**
 * @enum {number}
 */
proto.aptos.transaction.testing1.v1.AccountSignature.Type = {
  TYPE_UNSPECIFIED: 0,
  TYPE_ED25519: 1,
  TYPE_MULTI_ED25519: 2
};

/**
 * optional Type type = 1;
 * @return {!proto.aptos.transaction.testing1.v1.AccountSignature.Type}
 */
proto.aptos.transaction.testing1.v1.AccountSignature.prototype.getType = function() {
  return /** @type {!proto.aptos.transaction.testing1.v1.AccountSignature.Type} */ (jspb.Message.getFieldWithDefault(this, 1, 0));
};


/**
 * @param {!proto.aptos.transaction.testing1.v1.AccountSignature.Type} value
 * @return {!proto.aptos.transaction.testing1.v1.AccountSignature} returns this
 */
proto.aptos.transaction.testing1.v1.AccountSignature.prototype.setType = function(value) {
  return jspb.Message.setProto3EnumField(this, 1, value);
};


/**
 * optional Ed25519Signature ed25519 = 2;
 * @return {?proto.aptos.transaction.testing1.v1.Ed25519Signature}
 */
proto.aptos.transaction.testing1.v1.AccountSignature.prototype.getEd25519 = function() {
  return /** @type{?proto.aptos.transaction.testing1.v1.Ed25519Signature} */ (
    jspb.Message.getWrapperField(this, proto.aptos.transaction.testing1.v1.Ed25519Signature, 2));
};


/**
 * @param {?proto.aptos.transaction.testing1.v1.Ed25519Signature|undefined} value
 * @return {!proto.aptos.transaction.testing1.v1.AccountSignature} returns this
*/
proto.aptos.transaction.testing1.v1.AccountSignature.prototype.setEd25519 = function(value) {
  return jspb.Message.setOneofWrapperField(this, 2, proto.aptos.transaction.testing1.v1.AccountSignature.oneofGroups_[0], value);
};


/**
 * Clears the message field making it undefined.
 * @return {!proto.aptos.transaction.testing1.v1.AccountSignature} returns this
 */
proto.aptos.transaction.testing1.v1.AccountSignature.prototype.clearEd25519 = function() {
  return this.setEd25519(undefined);
};


/**
 * Returns whether this field is set.
 * @return {boolean}
 */
proto.aptos.transaction.testing1.v1.AccountSignature.prototype.hasEd25519 = function() {
  return jspb.Message.getField(this, 2) != null;
};


/**
 * optional MultiEd25519Signature multi_ed25519 = 3;
 * @return {?proto.aptos.transaction.testing1.v1.MultiEd25519Signature}
 */
proto.aptos.transaction.testing1.v1.AccountSignature.prototype.getMultiEd25519 = function() {
  return /** @type{?proto.aptos.transaction.testing1.v1.MultiEd25519Signature} */ (
    jspb.Message.getWrapperField(this, proto.aptos.transaction.testing1.v1.MultiEd25519Signature, 3));
};


/**
 * @param {?proto.aptos.transaction.testing1.v1.MultiEd25519Signature|undefined} value
 * @return {!proto.aptos.transaction.testing1.v1.AccountSignature} returns this
*/
proto.aptos.transaction.testing1.v1.AccountSignature.prototype.setMultiEd25519 = function(value) {
  return jspb.Message.setOneofWrapperField(this, 3, proto.aptos.transaction.testing1.v1.AccountSignature.oneofGroups_[0], value);
};


/**
 * Clears the message field making it undefined.
 * @return {!proto.aptos.transaction.testing1.v1.AccountSignature} returns this
 */
proto.aptos.transaction.testing1.v1.AccountSignature.prototype.clearMultiEd25519 = function() {
  return this.setMultiEd25519(undefined);
};


/**
 * Returns whether this field is set.
 * @return {boolean}
 */
proto.aptos.transaction.testing1.v1.AccountSignature.prototype.hasMultiEd25519 = function() {
  return jspb.Message.getField(this, 3) != null;
};


/**
 * @enum {number}
 */
proto.aptos.transaction.testing1.v1.MoveTypes = {
  MOVE_TYPES_UNSPECIFIED: 0,
  MOVE_TYPES_BOOL: 1,
  MOVE_TYPES_U8: 2,
  MOVE_TYPES_U16: 12,
  MOVE_TYPES_U32: 13,
  MOVE_TYPES_U64: 3,
  MOVE_TYPES_U128: 4,
  MOVE_TYPES_U256: 14,
  MOVE_TYPES_ADDRESS: 5,
  MOVE_TYPES_SIGNER: 6,
  MOVE_TYPES_VECTOR: 7,
  MOVE_TYPES_STRUCT: 8,
  MOVE_TYPES_GENERIC_TYPE_PARAM: 9,
  MOVE_TYPES_REFERENCE: 10,
  MOVE_TYPES_UNPARSABLE: 11
};

/**
 * @enum {number}
 */
proto.aptos.transaction.testing1.v1.MoveAbility = {
  MOVE_ABILITY_UNSPECIFIED: 0,
  MOVE_ABILITY_COPY: 1,
  MOVE_ABILITY_DROP: 2,
  MOVE_ABILITY_STORE: 3,
  MOVE_ABILITY_KEY: 4
};

goog.object.extend(exports, proto.aptos.transaction.testing1.v1);
