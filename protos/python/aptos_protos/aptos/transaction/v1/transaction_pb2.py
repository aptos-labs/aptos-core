# -*- coding: utf-8 -*-
# Generated by the protocol buffer compiler.  DO NOT EDIT!
# source: aptos/transaction/v1/transaction.proto
"""Generated protocol buffer code."""
from google.protobuf import descriptor as _descriptor
from google.protobuf import descriptor_pool as _descriptor_pool
from google.protobuf import symbol_database as _symbol_database
from google.protobuf.internal import builder as _builder

# @@protoc_insertion_point(imports)

_sym_db = _symbol_database.Default()


from aptos.util.timestamp import (
    timestamp_pb2 as aptos_dot_util_dot_timestamp_dot_timestamp__pb2,
)

DESCRIPTOR = _descriptor_pool.Default().AddSerializedFile(
    b'\n&aptos/transaction/v1/transaction.proto\x12\x14\x61ptos.transaction.v1\x1a$aptos/util/timestamp/timestamp.proto"\x9a\x01\n\x05\x42lock\x12\x32\n\ttimestamp\x18\x01 \x01(\x0b\x32\x1f.aptos.util.timestamp.Timestamp\x12\x12\n\x06height\x18\x02 \x01(\x04\x42\x02\x30\x01\x12\x37\n\x0ctransactions\x18\x03 \x03(\x0b\x32!.aptos.transaction.v1.Transaction\x12\x10\n\x08\x63hain_id\x18\x04 \x01(\r"\xda\x07\n\x0bTransaction\x12\x32\n\ttimestamp\x18\x01 \x01(\x0b\x32\x1f.aptos.util.timestamp.Timestamp\x12\x13\n\x07version\x18\x02 \x01(\x04\x42\x02\x30\x01\x12\x33\n\x04info\x18\x03 \x01(\x0b\x32%.aptos.transaction.v1.TransactionInfo\x12\x11\n\x05\x65poch\x18\x04 \x01(\x04\x42\x02\x30\x01\x12\x18\n\x0c\x62lock_height\x18\x05 \x01(\x04\x42\x02\x30\x01\x12?\n\x04type\x18\x06 \x01(\x0e\x32\x31.aptos.transaction.v1.Transaction.TransactionType\x12H\n\x0e\x62lock_metadata\x18\x07 \x01(\x0b\x32..aptos.transaction.v1.BlockMetadataTransactionH\x00\x12;\n\x07genesis\x18\x08 \x01(\x0b\x32(.aptos.transaction.v1.GenesisTransactionH\x00\x12L\n\x10state_checkpoint\x18\t \x01(\x0b\x32\x30.aptos.transaction.v1.StateCheckpointTransactionH\x00\x12\x35\n\x04user\x18\n \x01(\x0b\x32%.aptos.transaction.v1.UserTransactionH\x00\x12?\n\tvalidator\x18\x15 \x01(\x0b\x32*.aptos.transaction.v1.ValidatorTransactionH\x00\x12H\n\x0e\x62lock_epilogue\x18\x17 \x01(\x0b\x32..aptos.transaction.v1.BlockEpilogueTransactionH\x00\x12<\n\tsize_info\x18\x16 \x01(\x0b\x32).aptos.transaction.v1.TransactionSizeInfo"\xfd\x01\n\x0fTransactionType\x12 \n\x1cTRANSACTION_TYPE_UNSPECIFIED\x10\x00\x12\x1c\n\x18TRANSACTION_TYPE_GENESIS\x10\x01\x12#\n\x1fTRANSACTION_TYPE_BLOCK_METADATA\x10\x02\x12%\n!TRANSACTION_TYPE_STATE_CHECKPOINT\x10\x03\x12\x19\n\x15TRANSACTION_TYPE_USER\x10\x04\x12\x1e\n\x1aTRANSACTION_TYPE_VALIDATOR\x10\x14\x12#\n\x1fTRANSACTION_TYPE_BLOCK_EPILOGUE\x10\x15\x42\n\n\x08txn_data"\xbe\x01\n\x18\x42lockMetadataTransaction\x12\n\n\x02id\x18\x01 \x01(\t\x12\x11\n\x05round\x18\x02 \x01(\x04\x42\x02\x30\x01\x12+\n\x06\x65vents\x18\x03 \x03(\x0b\x32\x1b.aptos.transaction.v1.Event\x12#\n\x1bprevious_block_votes_bitvec\x18\x04 \x01(\x0c\x12\x10\n\x08proposer\x18\x05 \x01(\t\x12\x1f\n\x17\x66\x61iled_proposer_indices\x18\x06 \x03(\r"r\n\x12GenesisTransaction\x12/\n\x07payload\x18\x01 \x01(\x0b\x32\x1e.aptos.transaction.v1.WriteSet\x12+\n\x06\x65vents\x18\x02 \x03(\x0b\x32\x1b.aptos.transaction.v1.Event"\x1c\n\x1aStateCheckpointTransaction"\xcd\n\n\x14ValidatorTransaction\x12[\n\x13observed_jwk_update\x18\x01 \x01(\x0b\x32<.aptos.transaction.v1.ValidatorTransaction.ObservedJwkUpdateH\x00\x12J\n\ndkg_update\x18\x02 \x01(\x0b\x32\x34.aptos.transaction.v1.ValidatorTransaction.DkgUpdateH\x00\x1a\xc4\x07\n\x11ObservedJwkUpdate\x12s\n\x17quorum_certified_update\x18\x01 \x01(\x0b\x32R.aptos.transaction.v1.ValidatorTransaction.ObservedJwkUpdate.QuorumCertifiedUpdate\x1a\x8d\x04\n\x14\x45xportedProviderJWKs\x12\x0e\n\x06issuer\x18\x01 \x01(\t\x12\x0f\n\x07version\x18\x02 \x01(\x04\x12\x63\n\x04jwks\x18\x03 \x03(\x0b\x32U.aptos.transaction.v1.ValidatorTransaction.ObservedJwkUpdate.ExportedProviderJWKs.JWK\x1a\xee\x02\n\x03JWK\x12\x7f\n\x0funsupported_jwk\x18\x01 \x01(\x0b\x32\x64.aptos.transaction.v1.ValidatorTransaction.ObservedJwkUpdate.ExportedProviderJWKs.JWK.UnsupportedJWKH\x00\x12h\n\x03rsa\x18\x02 \x01(\x0b\x32Y.aptos.transaction.v1.ValidatorTransaction.ObservedJwkUpdate.ExportedProviderJWKs.JWK.RSAH\x00\x1a\x42\n\x03RSA\x12\x0b\n\x03kid\x18\x01 \x01(\t\x12\x0b\n\x03kty\x18\x02 \x01(\t\x12\x0b\n\x03\x61lg\x18\x03 \x01(\t\x12\t\n\x01\x65\x18\x04 \x01(\t\x12\t\n\x01n\x18\x05 \x01(\t\x1a-\n\x0eUnsupportedJWK\x12\n\n\x02id\x18\x01 \x01(\x0c\x12\x0f\n\x07payload\x18\x02 \x01(\x0c\x42\t\n\x07JwkType\x1a\x41\n\x1a\x45xportedAggregateSignature\x12\x16\n\x0esigner_indices\x18\x01 \x03(\x04\x12\x0b\n\x03sig\x18\x02 \x01(\x0c\x1a\xe6\x01\n\x15QuorumCertifiedUpdate\x12\x61\n\x06update\x18\x01 \x01(\x0b\x32Q.aptos.transaction.v1.ValidatorTransaction.ObservedJwkUpdate.ExportedProviderJWKs\x12j\n\tmulti_sig\x18\x02 \x01(\x0b\x32W.aptos.transaction.v1.ValidatorTransaction.ObservedJwkUpdate.ExportedAggregateSignature\x1a\xa8\x01\n\tDkgUpdate\x12Z\n\x0e\x64kg_transcript\x18\x01 \x01(\x0b\x32\x42.aptos.transaction.v1.ValidatorTransaction.DkgUpdate.DkgTranscript\x1a?\n\rDkgTranscript\x12\r\n\x05\x65poch\x18\x01 \x01(\x04\x12\x0e\n\x06\x61uthor\x18\x02 \x01(\t\x12\x0f\n\x07payload\x18\x03 \x01(\x0c\x42\x1a\n\x18ValidatorTransactionType"n\n\x18\x42lockEpilogueTransaction\x12?\n\x0e\x62lock_end_info\x18\x01 \x01(\x0b\x32".aptos.transaction.v1.BlockEndInfoH\x00\x88\x01\x01\x42\x11\n\x0f_block_end_info"\x9e\x01\n\x0c\x42lockEndInfo\x12\x1f\n\x17\x62lock_gas_limit_reached\x18\x01 \x01(\x08\x12"\n\x1a\x62lock_output_limit_reached\x18\x02 \x01(\x08\x12\'\n\x1f\x62lock_effective_block_gas_units\x18\x03 \x01(\x04\x12 \n\x18\x62lock_approx_output_size\x18\x04 \x01(\x04"}\n\x0fUserTransaction\x12=\n\x07request\x18\x01 \x01(\x0b\x32,.aptos.transaction.v1.UserTransactionRequest\x12+\n\x06\x65vents\x18\x02 \x03(\x0b\x32\x1b.aptos.transaction.v1.Event"\x9f\x01\n\x05\x45vent\x12+\n\x03key\x18\x01 \x01(\x0b\x32\x1e.aptos.transaction.v1.EventKey\x12\x1b\n\x0fsequence_number\x18\x02 \x01(\x04\x42\x02\x30\x01\x12,\n\x04type\x18\x03 \x01(\x0b\x32\x1e.aptos.transaction.v1.MoveType\x12\x10\n\x08type_str\x18\x05 \x01(\t\x12\x0c\n\x04\x64\x61ta\x18\x04 \x01(\t"\xa1\x02\n\x0fTransactionInfo\x12\x0c\n\x04hash\x18\x01 \x01(\x0c\x12\x19\n\x11state_change_hash\x18\x02 \x01(\x0c\x12\x17\n\x0f\x65vent_root_hash\x18\x03 \x01(\x0c\x12"\n\x15state_checkpoint_hash\x18\x04 \x01(\x0cH\x00\x88\x01\x01\x12\x14\n\x08gas_used\x18\x05 \x01(\x04\x42\x02\x30\x01\x12\x0f\n\x07success\x18\x06 \x01(\x08\x12\x11\n\tvm_status\x18\x07 \x01(\t\x12\x1d\n\x15\x61\x63\x63umulator_root_hash\x18\x08 \x01(\x0c\x12\x35\n\x07\x63hanges\x18\t \x03(\x0b\x32$.aptos.transaction.v1.WriteSetChangeB\x18\n\x16_state_checkpoint_hash"@\n\x08\x45ventKey\x12\x1b\n\x0f\x63reation_number\x18\x01 \x01(\x04\x42\x02\x30\x01\x12\x17\n\x0f\x61\x63\x63ount_address\x18\x02 \x01(\t"\xb0\x02\n\x16UserTransactionRequest\x12\x0e\n\x06sender\x18\x01 \x01(\t\x12\x1b\n\x0fsequence_number\x18\x02 \x01(\x04\x42\x02\x30\x01\x12\x1a\n\x0emax_gas_amount\x18\x03 \x01(\x04\x42\x02\x30\x01\x12\x1a\n\x0egas_unit_price\x18\x04 \x01(\x04\x42\x02\x30\x01\x12\x42\n\x19\x65xpiration_timestamp_secs\x18\x05 \x01(\x0b\x32\x1f.aptos.util.timestamp.Timestamp\x12\x39\n\x07payload\x18\x06 \x01(\x0b\x32(.aptos.transaction.v1.TransactionPayload\x12\x32\n\tsignature\x18\x07 \x01(\x0b\x32\x1f.aptos.transaction.v1.Signature"\xda\x02\n\x08WriteSet\x12\x43\n\x0ewrite_set_type\x18\x01 \x01(\x0e\x32+.aptos.transaction.v1.WriteSet.WriteSetType\x12@\n\x10script_write_set\x18\x02 \x01(\x0b\x32$.aptos.transaction.v1.ScriptWriteSetH\x00\x12@\n\x10\x64irect_write_set\x18\x03 \x01(\x0b\x32$.aptos.transaction.v1.DirectWriteSetH\x00"x\n\x0cWriteSetType\x12\x1e\n\x1aWRITE_SET_TYPE_UNSPECIFIED\x10\x00\x12#\n\x1fWRITE_SET_TYPE_SCRIPT_WRITE_SET\x10\x01\x12#\n\x1fWRITE_SET_TYPE_DIRECT_WRITE_SET\x10\x02\x42\x0b\n\twrite_set"Y\n\x0eScriptWriteSet\x12\x12\n\nexecute_as\x18\x01 \x01(\t\x12\x33\n\x06script\x18\x02 \x01(\x0b\x32#.aptos.transaction.v1.ScriptPayload"}\n\x0e\x44irectWriteSet\x12>\n\x10write_set_change\x18\x01 \x03(\x0b\x32$.aptos.transaction.v1.WriteSetChange\x12+\n\x06\x65vents\x18\x02 \x03(\x0b\x32\x1b.aptos.transaction.v1.Event"\x89\x05\n\x0eWriteSetChange\x12\x37\n\x04type\x18\x01 \x01(\x0e\x32).aptos.transaction.v1.WriteSetChange.Type\x12;\n\rdelete_module\x18\x02 \x01(\x0b\x32".aptos.transaction.v1.DeleteModuleH\x00\x12?\n\x0f\x64\x65lete_resource\x18\x03 \x01(\x0b\x32$.aptos.transaction.v1.DeleteResourceH\x00\x12\x42\n\x11\x64\x65lete_table_item\x18\x04 \x01(\x0b\x32%.aptos.transaction.v1.DeleteTableItemH\x00\x12\x39\n\x0cwrite_module\x18\x05 \x01(\x0b\x32!.aptos.transaction.v1.WriteModuleH\x00\x12=\n\x0ewrite_resource\x18\x06 \x01(\x0b\x32#.aptos.transaction.v1.WriteResourceH\x00\x12@\n\x10write_table_item\x18\x07 \x01(\x0b\x32$.aptos.transaction.v1.WriteTableItemH\x00"\xb5\x01\n\x04Type\x12\x14\n\x10TYPE_UNSPECIFIED\x10\x00\x12\x16\n\x12TYPE_DELETE_MODULE\x10\x01\x12\x18\n\x14TYPE_DELETE_RESOURCE\x10\x02\x12\x1a\n\x16TYPE_DELETE_TABLE_ITEM\x10\x03\x12\x15\n\x11TYPE_WRITE_MODULE\x10\x04\x12\x17\n\x13TYPE_WRITE_RESOURCE\x10\x05\x12\x19\n\x15TYPE_WRITE_TABLE_ITEM\x10\x06\x42\x08\n\x06\x63hange"k\n\x0c\x44\x65leteModule\x12\x0f\n\x07\x61\x64\x64ress\x18\x01 \x01(\t\x12\x16\n\x0estate_key_hash\x18\x02 \x01(\x0c\x12\x32\n\x06module\x18\x03 \x01(\x0b\x32".aptos.transaction.v1.MoveModuleId"~\n\x0e\x44\x65leteResource\x12\x0f\n\x07\x61\x64\x64ress\x18\x01 \x01(\t\x12\x16\n\x0estate_key_hash\x18\x02 \x01(\x0c\x12\x31\n\x04type\x18\x03 \x01(\x0b\x32#.aptos.transaction.v1.MoveStructTag\x12\x10\n\x08type_str\x18\x04 \x01(\t"{\n\x0f\x44\x65leteTableItem\x12\x16\n\x0estate_key_hash\x18\x01 \x01(\x0c\x12\x0e\n\x06handle\x18\x02 \x01(\t\x12\x0b\n\x03key\x18\x03 \x01(\t\x12\x33\n\x04\x64\x61ta\x18\x04 \x01(\x0b\x32%.aptos.transaction.v1.DeleteTableData"0\n\x0f\x44\x65leteTableData\x12\x0b\n\x03key\x18\x01 \x01(\t\x12\x10\n\x08key_type\x18\x02 \x01(\t"n\n\x0bWriteModule\x12\x0f\n\x07\x61\x64\x64ress\x18\x01 \x01(\t\x12\x16\n\x0estate_key_hash\x18\x02 \x01(\x0c\x12\x36\n\x04\x64\x61ta\x18\x03 \x01(\x0b\x32(.aptos.transaction.v1.MoveModuleBytecode"\x8b\x01\n\rWriteResource\x12\x0f\n\x07\x61\x64\x64ress\x18\x01 \x01(\t\x12\x16\n\x0estate_key_hash\x18\x02 \x01(\x0c\x12\x31\n\x04type\x18\x03 \x01(\x0b\x32#.aptos.transaction.v1.MoveStructTag\x12\x10\n\x08type_str\x18\x04 \x01(\t\x12\x0c\n\x04\x64\x61ta\x18\x05 \x01(\t"R\n\x0eWriteTableData\x12\x0b\n\x03key\x18\x01 \x01(\t\x12\x10\n\x08key_type\x18\x02 \x01(\t\x12\r\n\x05value\x18\x03 \x01(\t\x12\x12\n\nvalue_type\x18\x04 \x01(\t"y\n\x0eWriteTableItem\x12\x16\n\x0estate_key_hash\x18\x01 \x01(\x0c\x12\x0e\n\x06handle\x18\x02 \x01(\t\x12\x0b\n\x03key\x18\x03 \x01(\t\x12\x32\n\x04\x64\x61ta\x18\x04 \x01(\x0b\x32$.aptos.transaction.v1.WriteTableData"\x8c\x04\n\x12TransactionPayload\x12;\n\x04type\x18\x01 \x01(\x0e\x32-.aptos.transaction.v1.TransactionPayload.Type\x12L\n\x16\x65ntry_function_payload\x18\x02 \x01(\x0b\x32*.aptos.transaction.v1.EntryFunctionPayloadH\x00\x12=\n\x0escript_payload\x18\x03 \x01(\x0b\x32#.aptos.transaction.v1.ScriptPayloadH\x00\x12\x42\n\x11write_set_payload\x18\x05 \x01(\x0b\x32%.aptos.transaction.v1.WriteSetPayloadH\x00\x12\x41\n\x10multisig_payload\x18\x06 \x01(\x0b\x32%.aptos.transaction.v1.MultisigPayloadH\x00"\x93\x01\n\x04Type\x12\x14\n\x10TYPE_UNSPECIFIED\x10\x00\x12\x1f\n\x1bTYPE_ENTRY_FUNCTION_PAYLOAD\x10\x01\x12\x17\n\x13TYPE_SCRIPT_PAYLOAD\x10\x02\x12\x1a\n\x16TYPE_WRITE_SET_PAYLOAD\x10\x04\x12\x19\n\x15TYPE_MULTISIG_PAYLOAD\x10\x05"\x04\x08\x03\x10\x03\x42\t\n\x07payloadJ\x04\x08\x04\x10\x05"\xb9\x01\n\x14\x45ntryFunctionPayload\x12\x37\n\x08\x66unction\x18\x01 \x01(\x0b\x32%.aptos.transaction.v1.EntryFunctionId\x12\x36\n\x0etype_arguments\x18\x02 \x03(\x0b\x32\x1e.aptos.transaction.v1.MoveType\x12\x11\n\targuments\x18\x03 \x03(\t\x12\x1d\n\x15\x65ntry_function_id_str\x18\x04 \x01(\t"W\n\x12MoveScriptBytecode\x12\x10\n\x08\x62ytecode\x18\x01 \x01(\x0c\x12/\n\x03\x61\x62i\x18\x02 \x01(\x0b\x32".aptos.transaction.v1.MoveFunction"\x92\x01\n\rScriptPayload\x12\x36\n\x04\x63ode\x18\x01 \x01(\x0b\x32(.aptos.transaction.v1.MoveScriptBytecode\x12\x36\n\x0etype_arguments\x18\x02 \x03(\x0b\x32\x1e.aptos.transaction.v1.MoveType\x12\x11\n\targuments\x18\x03 \x03(\t"\x97\x01\n\x0fMultisigPayload\x12\x18\n\x10multisig_address\x18\x01 \x01(\t\x12R\n\x13transaction_payload\x18\x02 \x01(\x0b\x32\x30.aptos.transaction.v1.MultisigTransactionPayloadH\x00\x88\x01\x01\x42\x16\n\x14_transaction_payload"\xf9\x01\n\x1aMultisigTransactionPayload\x12\x43\n\x04type\x18\x01 \x01(\x0e\x32\x35.aptos.transaction.v1.MultisigTransactionPayload.Type\x12L\n\x16\x65ntry_function_payload\x18\x02 \x01(\x0b\x32*.aptos.transaction.v1.EntryFunctionPayloadH\x00"=\n\x04Type\x12\x14\n\x10TYPE_UNSPECIFIED\x10\x00\x12\x1f\n\x1bTYPE_ENTRY_FUNCTION_PAYLOAD\x10\x01\x42\t\n\x07payload"U\n\x12MoveModuleBytecode\x12\x10\n\x08\x62ytecode\x18\x01 \x01(\x0c\x12-\n\x03\x61\x62i\x18\x02 \x01(\x0b\x32 .aptos.transaction.v1.MoveModule"\xd2\x01\n\nMoveModule\x12\x0f\n\x07\x61\x64\x64ress\x18\x01 \x01(\t\x12\x0c\n\x04name\x18\x02 \x01(\t\x12\x33\n\x07\x66riends\x18\x03 \x03(\x0b\x32".aptos.transaction.v1.MoveModuleId\x12=\n\x11\x65xposed_functions\x18\x04 \x03(\x0b\x32".aptos.transaction.v1.MoveFunction\x12\x31\n\x07structs\x18\x05 \x03(\x0b\x32 .aptos.transaction.v1.MoveStruct"\x92\x03\n\x0cMoveFunction\x12\x0c\n\x04name\x18\x01 \x01(\t\x12\x41\n\nvisibility\x18\x02 \x01(\x0e\x32-.aptos.transaction.v1.MoveFunction.Visibility\x12\x10\n\x08is_entry\x18\x03 \x01(\x08\x12O\n\x13generic_type_params\x18\x04 \x03(\x0b\x32\x32.aptos.transaction.v1.MoveFunctionGenericTypeParam\x12.\n\x06params\x18\x05 \x03(\x0b\x32\x1e.aptos.transaction.v1.MoveType\x12.\n\x06return\x18\x06 \x03(\x0b\x32\x1e.aptos.transaction.v1.MoveType"n\n\nVisibility\x12\x1a\n\x16VISIBILITY_UNSPECIFIED\x10\x00\x12\x16\n\x12VISIBILITY_PRIVATE\x10\x01\x12\x15\n\x11VISIBILITY_PUBLIC\x10\x02\x12\x15\n\x11VISIBILITY_FRIEND\x10\x03"\xe9\x01\n\nMoveStruct\x12\x0c\n\x04name\x18\x01 \x01(\t\x12\x11\n\tis_native\x18\x02 \x01(\x08\x12\x34\n\tabilities\x18\x03 \x03(\x0e\x32!.aptos.transaction.v1.MoveAbility\x12M\n\x13generic_type_params\x18\x04 \x03(\x0b\x32\x30.aptos.transaction.v1.MoveStructGenericTypeParam\x12\x35\n\x06\x66ields\x18\x05 \x03(\x0b\x32%.aptos.transaction.v1.MoveStructField"h\n\x1aMoveStructGenericTypeParam\x12\x36\n\x0b\x63onstraints\x18\x01 \x03(\x0e\x32!.aptos.transaction.v1.MoveAbility\x12\x12\n\nis_phantom\x18\x02 \x01(\x08"M\n\x0fMoveStructField\x12\x0c\n\x04name\x18\x01 \x01(\t\x12,\n\x04type\x18\x02 \x01(\x0b\x32\x1e.aptos.transaction.v1.MoveType"V\n\x1cMoveFunctionGenericTypeParam\x12\x36\n\x0b\x63onstraints\x18\x01 \x03(\x0e\x32!.aptos.transaction.v1.MoveAbility"\xf8\x02\n\x08MoveType\x12-\n\x04type\x18\x01 \x01(\x0e\x32\x1f.aptos.transaction.v1.MoveTypes\x12\x30\n\x06vector\x18\x03 \x01(\x0b\x32\x1e.aptos.transaction.v1.MoveTypeH\x00\x12\x35\n\x06struct\x18\x04 \x01(\x0b\x32#.aptos.transaction.v1.MoveStructTagH\x00\x12"\n\x18generic_type_param_index\x18\x05 \x01(\rH\x00\x12\x41\n\treference\x18\x06 \x01(\x0b\x32,.aptos.transaction.v1.MoveType.ReferenceTypeH\x00\x12\x14\n\nunparsable\x18\x07 \x01(\tH\x00\x1aL\n\rReferenceType\x12\x0f\n\x07mutable\x18\x01 \x01(\x08\x12*\n\x02to\x18\x02 \x01(\x0b\x32\x1e.aptos.transaction.v1.MoveTypeB\t\n\x07\x63ontent"D\n\x0fWriteSetPayload\x12\x31\n\twrite_set\x18\x01 \x01(\x0b\x32\x1e.aptos.transaction.v1.WriteSet"S\n\x0f\x45ntryFunctionId\x12\x32\n\x06module\x18\x01 \x01(\x0b\x32".aptos.transaction.v1.MoveModuleId\x12\x0c\n\x04name\x18\x02 \x01(\t"-\n\x0cMoveModuleId\x12\x0f\n\x07\x61\x64\x64ress\x18\x01 \x01(\t\x12\x0c\n\x04name\x18\x02 \x01(\t"{\n\rMoveStructTag\x12\x0f\n\x07\x61\x64\x64ress\x18\x01 \x01(\t\x12\x0e\n\x06module\x18\x02 \x01(\t\x12\x0c\n\x04name\x18\x03 \x01(\t\x12;\n\x13generic_type_params\x18\x04 \x03(\x0b\x32\x1e.aptos.transaction.v1.MoveType"\x9b\x04\n\tSignature\x12\x32\n\x04type\x18\x01 \x01(\x0e\x32$.aptos.transaction.v1.Signature.Type\x12\x39\n\x07\x65\x64\x32\x35\x35\x31\x39\x18\x02 \x01(\x0b\x32&.aptos.transaction.v1.Ed25519SignatureH\x00\x12\x44\n\rmulti_ed25519\x18\x03 \x01(\x0b\x32+.aptos.transaction.v1.MultiEd25519SignatureH\x00\x12@\n\x0bmulti_agent\x18\x04 \x01(\x0b\x32).aptos.transaction.v1.MultiAgentSignatureH\x00\x12<\n\tfee_payer\x18\x05 \x01(\x0b\x32\'.aptos.transaction.v1.FeePayerSignatureH\x00\x12;\n\rsingle_sender\x18\x07 \x01(\x0b\x32".aptos.transaction.v1.SingleSenderH\x00"\x8e\x01\n\x04Type\x12\x14\n\x10TYPE_UNSPECIFIED\x10\x00\x12\x10\n\x0cTYPE_ED25519\x10\x01\x12\x16\n\x12TYPE_MULTI_ED25519\x10\x02\x12\x14\n\x10TYPE_MULTI_AGENT\x10\x03\x12\x12\n\x0eTYPE_FEE_PAYER\x10\x04\x12\x16\n\x12TYPE_SINGLE_SENDER\x10\x06"\x04\x08\x05\x10\x05\x42\x0b\n\tsignature"9\n\x10\x45\x64\x32\x35\x35\x31\x39Signature\x12\x12\n\npublic_key\x18\x01 \x01(\x0c\x12\x11\n\tsignature\x18\x02 \x01(\x0c"o\n\x15MultiEd25519Signature\x12\x13\n\x0bpublic_keys\x18\x01 \x03(\x0c\x12\x12\n\nsignatures\x18\x02 \x03(\x0c\x12\x11\n\tthreshold\x18\x03 \x01(\r\x12\x1a\n\x12public_key_indices\x18\x04 \x03(\r"\xb4\x01\n\x13MultiAgentSignature\x12\x36\n\x06sender\x18\x01 \x01(\x0b\x32&.aptos.transaction.v1.AccountSignature\x12"\n\x1asecondary_signer_addresses\x18\x02 \x03(\t\x12\x41\n\x11secondary_signers\x18\x03 \x03(\x0b\x32&.aptos.transaction.v1.AccountSignature"\x8f\x02\n\x11\x46\x65\x65PayerSignature\x12\x36\n\x06sender\x18\x01 \x01(\x0b\x32&.aptos.transaction.v1.AccountSignature\x12"\n\x1asecondary_signer_addresses\x18\x02 \x03(\t\x12\x41\n\x11secondary_signers\x18\x03 \x03(\x0b\x32&.aptos.transaction.v1.AccountSignature\x12\x19\n\x11\x66\x65\x65_payer_address\x18\x04 \x01(\t\x12@\n\x10\x66\x65\x65_payer_signer\x18\x05 \x01(\x0b\x32&.aptos.transaction.v1.AccountSignature"\xcf\x01\n\x0c\x41nyPublicKey\x12\x35\n\x04type\x18\x01 \x01(\x0e\x32\'.aptos.transaction.v1.AnyPublicKey.Type\x12\x12\n\npublic_key\x18\x02 \x01(\x0c"t\n\x04Type\x12\x14\n\x10TYPE_UNSPECIFIED\x10\x00\x12\x10\n\x0cTYPE_ED25519\x10\x01\x12\x18\n\x14TYPE_SECP256K1_ECDSA\x10\x02\x12\x18\n\x14TYPE_SECP256R1_ECDSA\x10\x03\x12\x10\n\x0cTYPE_KEYLESS\x10\x04"\xb9\x03\n\x0c\x41nySignature\x12\x35\n\x04type\x18\x01 \x01(\x0e\x32\'.aptos.transaction.v1.AnySignature.Type\x12\x15\n\tsignature\x18\x02 \x01(\x0c\x42\x02\x18\x01\x12\x30\n\x07\x65\x64\x32\x35\x35\x31\x39\x18\x03 \x01(\x0b\x32\x1d.aptos.transaction.v1.Ed25519H\x00\x12?\n\x0fsecp256k1_ecdsa\x18\x04 \x01(\x0b\x32$.aptos.transaction.v1.Secp256k1EcdsaH\x00\x12\x32\n\x08webauthn\x18\x05 \x01(\x0b\x32\x1e.aptos.transaction.v1.WebAuthnH\x00\x12\x30\n\x07keyless\x18\x06 \x01(\x0b\x32\x1d.aptos.transaction.v1.KeylessH\x00"m\n\x04Type\x12\x14\n\x10TYPE_UNSPECIFIED\x10\x00\x12\x10\n\x0cTYPE_ED25519\x10\x01\x12\x18\n\x14TYPE_SECP256K1_ECDSA\x10\x02\x12\x11\n\rTYPE_WEBAUTHN\x10\x03\x12\x10\n\x0cTYPE_KEYLESS\x10\x04\x42\x13\n\x11signature_variant"\x1c\n\x07\x45\x64\x32\x35\x35\x31\x39\x12\x11\n\tsignature\x18\x01 \x01(\x0c"#\n\x0eSecp256k1Ecdsa\x12\x11\n\tsignature\x18\x01 \x01(\x0c"\x1d\n\x08WebAuthn\x12\x11\n\tsignature\x18\x01 \x01(\x0c"\x1c\n\x07Keyless\x12\x11\n\tsignature\x18\x01 \x01(\x0c"\x83\x01\n\x12SingleKeySignature\x12\x36\n\npublic_key\x18\x01 \x01(\x0b\x32".aptos.transaction.v1.AnyPublicKey\x12\x35\n\tsignature\x18\x02 \x01(\x0b\x32".aptos.transaction.v1.AnySignature"X\n\x10IndexedSignature\x12\r\n\x05index\x18\x01 \x01(\r\x12\x35\n\tsignature\x18\x02 \x01(\x0b\x32".aptos.transaction.v1.AnySignature"\xa5\x01\n\x11MultiKeySignature\x12\x37\n\x0bpublic_keys\x18\x01 \x03(\x0b\x32".aptos.transaction.v1.AnyPublicKey\x12:\n\nsignatures\x18\x02 \x03(\x0b\x32&.aptos.transaction.v1.IndexedSignature\x12\x1b\n\x13signatures_required\x18\x03 \x01(\r"F\n\x0cSingleSender\x12\x36\n\x06sender\x18\x01 \x01(\x0b\x32&.aptos.transaction.v1.AccountSignature"\xe4\x03\n\x10\x41\x63\x63ountSignature\x12\x39\n\x04type\x18\x01 \x01(\x0e\x32+.aptos.transaction.v1.AccountSignature.Type\x12\x39\n\x07\x65\x64\x32\x35\x35\x31\x39\x18\x02 \x01(\x0b\x32&.aptos.transaction.v1.Ed25519SignatureH\x00\x12\x44\n\rmulti_ed25519\x18\x03 \x01(\x0b\x32+.aptos.transaction.v1.MultiEd25519SignatureH\x00\x12H\n\x14single_key_signature\x18\x05 \x01(\x0b\x32(.aptos.transaction.v1.SingleKeySignatureH\x00\x12\x46\n\x13multi_key_signature\x18\x06 \x01(\x0b\x32\'.aptos.transaction.v1.MultiKeySignatureH\x00"u\n\x04Type\x12\x14\n\x10TYPE_UNSPECIFIED\x10\x00\x12\x10\n\x0cTYPE_ED25519\x10\x01\x12\x16\n\x12TYPE_MULTI_ED25519\x10\x02\x12\x13\n\x0fTYPE_SINGLE_KEY\x10\x04\x12\x12\n\x0eTYPE_MULTI_KEY\x10\x05"\x04\x08\x03\x10\x03\x42\x0b\n\tsignature"\xb1\x01\n\x13TransactionSizeInfo\x12\x19\n\x11transaction_bytes\x18\x01 \x01(\r\x12<\n\x0f\x65vent_size_info\x18\x02 \x03(\x0b\x32#.aptos.transaction.v1.EventSizeInfo\x12\x41\n\x12write_op_size_info\x18\x03 \x03(\x0b\x32%.aptos.transaction.v1.WriteOpSizeInfo"<\n\rEventSizeInfo\x12\x16\n\x0etype_tag_bytes\x18\x01 \x01(\r\x12\x13\n\x0btotal_bytes\x18\x02 \x01(\r"9\n\x0fWriteOpSizeInfo\x12\x11\n\tkey_bytes\x18\x01 \x01(\r\x12\x13\n\x0bvalue_bytes\x18\x02 \x01(\r*\xea\x02\n\tMoveTypes\x12\x1a\n\x16MOVE_TYPES_UNSPECIFIED\x10\x00\x12\x13\n\x0fMOVE_TYPES_BOOL\x10\x01\x12\x11\n\rMOVE_TYPES_U8\x10\x02\x12\x12\n\x0eMOVE_TYPES_U16\x10\x0c\x12\x12\n\x0eMOVE_TYPES_U32\x10\r\x12\x12\n\x0eMOVE_TYPES_U64\x10\x03\x12\x13\n\x0fMOVE_TYPES_U128\x10\x04\x12\x13\n\x0fMOVE_TYPES_U256\x10\x0e\x12\x16\n\x12MOVE_TYPES_ADDRESS\x10\x05\x12\x15\n\x11MOVE_TYPES_SIGNER\x10\x06\x12\x15\n\x11MOVE_TYPES_VECTOR\x10\x07\x12\x15\n\x11MOVE_TYPES_STRUCT\x10\x08\x12!\n\x1dMOVE_TYPES_GENERIC_TYPE_PARAM\x10\t\x12\x18\n\x14MOVE_TYPES_REFERENCE\x10\n\x12\x19\n\x15MOVE_TYPES_UNPARSABLE\x10\x0b*\x87\x01\n\x0bMoveAbility\x12\x1c\n\x18MOVE_ABILITY_UNSPECIFIED\x10\x00\x12\x15\n\x11MOVE_ABILITY_COPY\x10\x01\x12\x15\n\x11MOVE_ABILITY_DROP\x10\x02\x12\x16\n\x12MOVE_ABILITY_STORE\x10\x03\x12\x14\n\x10MOVE_ABILITY_KEY\x10\x04\x62\x06proto3'
)

_globals = globals()
_builder.BuildMessageAndEnumDescriptors(DESCRIPTOR, _globals)
_builder.BuildTopDescriptorsAndMessages(
    DESCRIPTOR, "aptos.transaction.v1.transaction_pb2", _globals
)
if _descriptor._USE_C_DESCRIPTORS == False:
    DESCRIPTOR._options = None
    _BLOCK.fields_by_name["height"]._options = None
    _BLOCK.fields_by_name["height"]._serialized_options = b"0\001"
    _TRANSACTION.fields_by_name["version"]._options = None
    _TRANSACTION.fields_by_name["version"]._serialized_options = b"0\001"
    _TRANSACTION.fields_by_name["epoch"]._options = None
    _TRANSACTION.fields_by_name["epoch"]._serialized_options = b"0\001"
    _TRANSACTION.fields_by_name["block_height"]._options = None
    _TRANSACTION.fields_by_name["block_height"]._serialized_options = b"0\001"
    _BLOCKMETADATATRANSACTION.fields_by_name["round"]._options = None
    _BLOCKMETADATATRANSACTION.fields_by_name["round"]._serialized_options = b"0\001"
    _EVENT.fields_by_name["sequence_number"]._options = None
    _EVENT.fields_by_name["sequence_number"]._serialized_options = b"0\001"
    _TRANSACTIONINFO.fields_by_name["gas_used"]._options = None
    _TRANSACTIONINFO.fields_by_name["gas_used"]._serialized_options = b"0\001"
    _EVENTKEY.fields_by_name["creation_number"]._options = None
    _EVENTKEY.fields_by_name["creation_number"]._serialized_options = b"0\001"
    _USERTRANSACTIONREQUEST.fields_by_name["sequence_number"]._options = None
    _USERTRANSACTIONREQUEST.fields_by_name[
        "sequence_number"
    ]._serialized_options = b"0\001"
    _USERTRANSACTIONREQUEST.fields_by_name["max_gas_amount"]._options = None
    _USERTRANSACTIONREQUEST.fields_by_name[
        "max_gas_amount"
    ]._serialized_options = b"0\001"
    _USERTRANSACTIONREQUEST.fields_by_name["gas_unit_price"]._options = None
    _USERTRANSACTIONREQUEST.fields_by_name[
        "gas_unit_price"
    ]._serialized_options = b"0\001"
    _ANYSIGNATURE.fields_by_name["signature"]._options = None
    _ANYSIGNATURE.fields_by_name["signature"]._serialized_options = b"\030\001"
    _globals["_MOVETYPES"]._serialized_start = 12751
    _globals["_MOVETYPES"]._serialized_end = 13113
    _globals["_MOVEABILITY"]._serialized_start = 13116
    _globals["_MOVEABILITY"]._serialized_end = 13251
    _globals["_BLOCK"]._serialized_start = 103
    _globals["_BLOCK"]._serialized_end = 257
    _globals["_TRANSACTION"]._serialized_start = 260
    _globals["_TRANSACTION"]._serialized_end = 1246
    _globals["_TRANSACTION_TRANSACTIONTYPE"]._serialized_start = 981
    _globals["_TRANSACTION_TRANSACTIONTYPE"]._serialized_end = 1234
    _globals["_BLOCKMETADATATRANSACTION"]._serialized_start = 1249
    _globals["_BLOCKMETADATATRANSACTION"]._serialized_end = 1439
    _globals["_GENESISTRANSACTION"]._serialized_start = 1441
    _globals["_GENESISTRANSACTION"]._serialized_end = 1555
    _globals["_STATECHECKPOINTTRANSACTION"]._serialized_start = 1557
    _globals["_STATECHECKPOINTTRANSACTION"]._serialized_end = 1585
    _globals["_VALIDATORTRANSACTION"]._serialized_start = 1588
    _globals["_VALIDATORTRANSACTION"]._serialized_end = 2945
    _globals["_VALIDATORTRANSACTION_OBSERVEDJWKUPDATE"]._serialized_start = 1782
    _globals["_VALIDATORTRANSACTION_OBSERVEDJWKUPDATE"]._serialized_end = 2746
    _globals[
        "_VALIDATORTRANSACTION_OBSERVEDJWKUPDATE_EXPORTEDPROVIDERJWKS"
    ]._serialized_start = 1921
    _globals[
        "_VALIDATORTRANSACTION_OBSERVEDJWKUPDATE_EXPORTEDPROVIDERJWKS"
    ]._serialized_end = 2446
    _globals[
        "_VALIDATORTRANSACTION_OBSERVEDJWKUPDATE_EXPORTEDPROVIDERJWKS_JWK"
    ]._serialized_start = 2080
    _globals[
        "_VALIDATORTRANSACTION_OBSERVEDJWKUPDATE_EXPORTEDPROVIDERJWKS_JWK"
    ]._serialized_end = 2446
    _globals[
        "_VALIDATORTRANSACTION_OBSERVEDJWKUPDATE_EXPORTEDPROVIDERJWKS_JWK_RSA"
    ]._serialized_start = 2322
    _globals[
        "_VALIDATORTRANSACTION_OBSERVEDJWKUPDATE_EXPORTEDPROVIDERJWKS_JWK_RSA"
    ]._serialized_end = 2388
    _globals[
        "_VALIDATORTRANSACTION_OBSERVEDJWKUPDATE_EXPORTEDPROVIDERJWKS_JWK_UNSUPPORTEDJWK"
    ]._serialized_start = 2390
    _globals[
        "_VALIDATORTRANSACTION_OBSERVEDJWKUPDATE_EXPORTEDPROVIDERJWKS_JWK_UNSUPPORTEDJWK"
    ]._serialized_end = 2435
    _globals[
        "_VALIDATORTRANSACTION_OBSERVEDJWKUPDATE_EXPORTEDAGGREGATESIGNATURE"
    ]._serialized_start = 2448
    _globals[
        "_VALIDATORTRANSACTION_OBSERVEDJWKUPDATE_EXPORTEDAGGREGATESIGNATURE"
    ]._serialized_end = 2513
    _globals[
        "_VALIDATORTRANSACTION_OBSERVEDJWKUPDATE_QUORUMCERTIFIEDUPDATE"
    ]._serialized_start = 2516
    _globals[
        "_VALIDATORTRANSACTION_OBSERVEDJWKUPDATE_QUORUMCERTIFIEDUPDATE"
    ]._serialized_end = 2746
    _globals["_VALIDATORTRANSACTION_DKGUPDATE"]._serialized_start = 2749
    _globals["_VALIDATORTRANSACTION_DKGUPDATE"]._serialized_end = 2917
    _globals["_VALIDATORTRANSACTION_DKGUPDATE_DKGTRANSCRIPT"]._serialized_start = 2854
    _globals["_VALIDATORTRANSACTION_DKGUPDATE_DKGTRANSCRIPT"]._serialized_end = 2917
    _globals["_BLOCKEPILOGUETRANSACTION"]._serialized_start = 2947
    _globals["_BLOCKEPILOGUETRANSACTION"]._serialized_end = 3057
    _globals["_BLOCKENDINFO"]._serialized_start = 3060
    _globals["_BLOCKENDINFO"]._serialized_end = 3218
    _globals["_USERTRANSACTION"]._serialized_start = 3220
    _globals["_USERTRANSACTION"]._serialized_end = 3345
    _globals["_EVENT"]._serialized_start = 3348
    _globals["_EVENT"]._serialized_end = 3507
    _globals["_TRANSACTIONINFO"]._serialized_start = 3510
    _globals["_TRANSACTIONINFO"]._serialized_end = 3799
    _globals["_EVENTKEY"]._serialized_start = 3801
    _globals["_EVENTKEY"]._serialized_end = 3865
    _globals["_USERTRANSACTIONREQUEST"]._serialized_start = 3868
    _globals["_USERTRANSACTIONREQUEST"]._serialized_end = 4172
    _globals["_WRITESET"]._serialized_start = 4175
    _globals["_WRITESET"]._serialized_end = 4521
    _globals["_WRITESET_WRITESETTYPE"]._serialized_start = 4388
    _globals["_WRITESET_WRITESETTYPE"]._serialized_end = 4508
    _globals["_SCRIPTWRITESET"]._serialized_start = 4523
    _globals["_SCRIPTWRITESET"]._serialized_end = 4612
    _globals["_DIRECTWRITESET"]._serialized_start = 4614
    _globals["_DIRECTWRITESET"]._serialized_end = 4739
    _globals["_WRITESETCHANGE"]._serialized_start = 4742
    _globals["_WRITESETCHANGE"]._serialized_end = 5391
    _globals["_WRITESETCHANGE_TYPE"]._serialized_start = 5200
    _globals["_WRITESETCHANGE_TYPE"]._serialized_end = 5381
    _globals["_DELETEMODULE"]._serialized_start = 5393
    _globals["_DELETEMODULE"]._serialized_end = 5500
    _globals["_DELETERESOURCE"]._serialized_start = 5502
    _globals["_DELETERESOURCE"]._serialized_end = 5628
    _globals["_DELETETABLEITEM"]._serialized_start = 5630
    _globals["_DELETETABLEITEM"]._serialized_end = 5753
    _globals["_DELETETABLEDATA"]._serialized_start = 5755
    _globals["_DELETETABLEDATA"]._serialized_end = 5803
    _globals["_WRITEMODULE"]._serialized_start = 5805
    _globals["_WRITEMODULE"]._serialized_end = 5915
    _globals["_WRITERESOURCE"]._serialized_start = 5918
    _globals["_WRITERESOURCE"]._serialized_end = 6057
    _globals["_WRITETABLEDATA"]._serialized_start = 6059
    _globals["_WRITETABLEDATA"]._serialized_end = 6141
    _globals["_WRITETABLEITEM"]._serialized_start = 6143
    _globals["_WRITETABLEITEM"]._serialized_end = 6264
    _globals["_TRANSACTIONPAYLOAD"]._serialized_start = 6267
    _globals["_TRANSACTIONPAYLOAD"]._serialized_end = 6791
    _globals["_TRANSACTIONPAYLOAD_TYPE"]._serialized_start = 6627
    _globals["_TRANSACTIONPAYLOAD_TYPE"]._serialized_end = 6774
    _globals["_ENTRYFUNCTIONPAYLOAD"]._serialized_start = 6794
    _globals["_ENTRYFUNCTIONPAYLOAD"]._serialized_end = 6979
    _globals["_MOVESCRIPTBYTECODE"]._serialized_start = 6981
    _globals["_MOVESCRIPTBYTECODE"]._serialized_end = 7068
    _globals["_SCRIPTPAYLOAD"]._serialized_start = 7071
    _globals["_SCRIPTPAYLOAD"]._serialized_end = 7217
    _globals["_MULTISIGPAYLOAD"]._serialized_start = 7220
    _globals["_MULTISIGPAYLOAD"]._serialized_end = 7371
    _globals["_MULTISIGTRANSACTIONPAYLOAD"]._serialized_start = 7374
    _globals["_MULTISIGTRANSACTIONPAYLOAD"]._serialized_end = 7623
    _globals["_MULTISIGTRANSACTIONPAYLOAD_TYPE"]._serialized_start = 6627
    _globals["_MULTISIGTRANSACTIONPAYLOAD_TYPE"]._serialized_end = 6688
    _globals["_MOVEMODULEBYTECODE"]._serialized_start = 7625
    _globals["_MOVEMODULEBYTECODE"]._serialized_end = 7710
    _globals["_MOVEMODULE"]._serialized_start = 7713
    _globals["_MOVEMODULE"]._serialized_end = 7923
    _globals["_MOVEFUNCTION"]._serialized_start = 7926
    _globals["_MOVEFUNCTION"]._serialized_end = 8328
    _globals["_MOVEFUNCTION_VISIBILITY"]._serialized_start = 8218
    _globals["_MOVEFUNCTION_VISIBILITY"]._serialized_end = 8328
    _globals["_MOVESTRUCT"]._serialized_start = 8331
    _globals["_MOVESTRUCT"]._serialized_end = 8564
    _globals["_MOVESTRUCTGENERICTYPEPARAM"]._serialized_start = 8566
    _globals["_MOVESTRUCTGENERICTYPEPARAM"]._serialized_end = 8670
    _globals["_MOVESTRUCTFIELD"]._serialized_start = 8672
    _globals["_MOVESTRUCTFIELD"]._serialized_end = 8749
    _globals["_MOVEFUNCTIONGENERICTYPEPARAM"]._serialized_start = 8751
    _globals["_MOVEFUNCTIONGENERICTYPEPARAM"]._serialized_end = 8837
    _globals["_MOVETYPE"]._serialized_start = 8840
    _globals["_MOVETYPE"]._serialized_end = 9216
    _globals["_MOVETYPE_REFERENCETYPE"]._serialized_start = 9129
    _globals["_MOVETYPE_REFERENCETYPE"]._serialized_end = 9205
    _globals["_WRITESETPAYLOAD"]._serialized_start = 9218
    _globals["_WRITESETPAYLOAD"]._serialized_end = 9286
    _globals["_ENTRYFUNCTIONID"]._serialized_start = 9288
    _globals["_ENTRYFUNCTIONID"]._serialized_end = 9371
    _globals["_MOVEMODULEID"]._serialized_start = 9373
    _globals["_MOVEMODULEID"]._serialized_end = 9418
    _globals["_MOVESTRUCTTAG"]._serialized_start = 9420
    _globals["_MOVESTRUCTTAG"]._serialized_end = 9543
    _globals["_SIGNATURE"]._serialized_start = 9546
    _globals["_SIGNATURE"]._serialized_end = 10085
    _globals["_SIGNATURE_TYPE"]._serialized_start = 9930
    _globals["_SIGNATURE_TYPE"]._serialized_end = 10072
    _globals["_ED25519SIGNATURE"]._serialized_start = 10087
    _globals["_ED25519SIGNATURE"]._serialized_end = 10144
    _globals["_MULTIED25519SIGNATURE"]._serialized_start = 10146
    _globals["_MULTIED25519SIGNATURE"]._serialized_end = 10257
    _globals["_MULTIAGENTSIGNATURE"]._serialized_start = 10260
    _globals["_MULTIAGENTSIGNATURE"]._serialized_end = 10440
    _globals["_FEEPAYERSIGNATURE"]._serialized_start = 10443
    _globals["_FEEPAYERSIGNATURE"]._serialized_end = 10714
    _globals["_ANYPUBLICKEY"]._serialized_start = 10717
    _globals["_ANYPUBLICKEY"]._serialized_end = 10924
    _globals["_ANYPUBLICKEY_TYPE"]._serialized_start = 10808
    _globals["_ANYPUBLICKEY_TYPE"]._serialized_end = 10924
    _globals["_ANYSIGNATURE"]._serialized_start = 10927
    _globals["_ANYSIGNATURE"]._serialized_end = 11368
    _globals["_ANYSIGNATURE_TYPE"]._serialized_start = 11238
    _globals["_ANYSIGNATURE_TYPE"]._serialized_end = 11347
    _globals["_ED25519"]._serialized_start = 11370
    _globals["_ED25519"]._serialized_end = 11398
    _globals["_SECP256K1ECDSA"]._serialized_start = 11400
    _globals["_SECP256K1ECDSA"]._serialized_end = 11435
    _globals["_WEBAUTHN"]._serialized_start = 11437
    _globals["_WEBAUTHN"]._serialized_end = 11466
    _globals["_KEYLESS"]._serialized_start = 11468
    _globals["_KEYLESS"]._serialized_end = 11496
    _globals["_SINGLEKEYSIGNATURE"]._serialized_start = 11499
    _globals["_SINGLEKEYSIGNATURE"]._serialized_end = 11630
    _globals["_INDEXEDSIGNATURE"]._serialized_start = 11632
    _globals["_INDEXEDSIGNATURE"]._serialized_end = 11720
    _globals["_MULTIKEYSIGNATURE"]._serialized_start = 11723
    _globals["_MULTIKEYSIGNATURE"]._serialized_end = 11888
    _globals["_SINGLESENDER"]._serialized_start = 11890
    _globals["_SINGLESENDER"]._serialized_end = 11960
    _globals["_ACCOUNTSIGNATURE"]._serialized_start = 11963
    _globals["_ACCOUNTSIGNATURE"]._serialized_end = 12447
    _globals["_ACCOUNTSIGNATURE_TYPE"]._serialized_start = 12317
    _globals["_ACCOUNTSIGNATURE_TYPE"]._serialized_end = 12434
    _globals["_TRANSACTIONSIZEINFO"]._serialized_start = 12450
    _globals["_TRANSACTIONSIZEINFO"]._serialized_end = 12627
    _globals["_EVENTSIZEINFO"]._serialized_start = 12629
    _globals["_EVENTSIZEINFO"]._serialized_end = 12689
    _globals["_WRITEOPSIZEINFO"]._serialized_start = 12691
    _globals["_WRITEOPSIZEINFO"]._serialized_end = 12748
# @@protoc_insertion_point(module_scope)
