import numpy as np
import numpy.linalg as la
import scipy as sp

data = [
    ["entry_point_Nop", 2963, 1.5, 0.0012, 0, 0],
    ["entry_point_BytesMakeOrChange { data_length: Some(32) }", 2426, 1.5, 0.099, 0.8, 0.1087],
    ["entry_point_StepDst", 2388, 1.5, 0.1894, 2.0096, 0.2178],
    ["entry_point_Loop { loop_count: Some(100000), loop_type: NoOp }", 27, 1.5, 2400.0128, 0, 0],
    ["entry_point_Loop { loop_count: Some(10000), loop_type: Arithmetic }", 44, 1.5, 1312.02, 0, 0],
    ["entry_point_CreateObjects { num_objects: 10, object_payload_size: 0 }", 666, 1.5, 4.8356, 8, 1.227],
    ["entry_point_CreateObjects { num_objects: 10, object_payload_size: 10240 }", 103, 1.5, 505.4636, 16, 11.527],
    ["entry_point_CreateObjects { num_objects: 100, object_payload_size: 0 }", 93, 1.5, 47.3516, 80, 12.27],
    ["entry_point_CreateObjects { num_objects: 100, object_payload_size: 10240 }", 43, 1.5, 629.9516, 160, 115.27],
    ["entry_point_InitializeVectorPicture { length: 40 }", 1605, 1.5, 2.6054, 2.4, 0.2531],
    ["entry_point_VectorPicture { length: 40 }", 2850, 1.5, 0.1048, 2.4192, 0.1405],
    ["entry_point_VectorPictureRead { length: 40 }", 2900, 1.5, 0.147, 2.4192, 0],
    ["entry_point_InitializeVectorPicture { length: 30720 }", 30, 1.5, 1438.4294, 2.4, 9.4575],
    ["entry_point_VectorPicture { length: 30720 }", 169, 1.5, 0.1048, 20.8512, 9.3449],
    ["entry_point_VectorPictureRead { length: 30720 }", 189, 1.5, 0.147, 20.8512, 0],
    ["entry_point_SmartTablePicture { length: 30720, num_points_per_txn: 200 }", 22, 4.254, 1623.39, 2.4192, 0.8106],
    ["entry_point_SmartTablePicture { length: 1048576, num_points_per_txn: 1024 }", 3, 19.086, 11957.2104, 2.4192, 3.3392],
    ["entry_point_TokenV1MintAndTransferFT", 1351, 1.5, 19.18316, 6.4384, 0.8813],
    ["entry_point_TokenV1MintAndTransferNFTSequential", 971, 1.5, 30.42634, 6.4384, 0.9953],
    # ["entry_point_TokenV2AmbassadorMint { multisig: false }", 1077, 1.5, 0, 0, 0],
    ["Transfer", 2032, 1.5, 2.249, 2.4192, 0.2482],
    ["CreateAccount", 1583, 1.5, 3.1536, 1.6, 0.247],
    ["PublishModule", 138, 27.734, 2.86256, 37.6064, 1.6832],

    # large db
    # ["entry_point_Nop", 2963, 1.5, 0.0012, 0, 0],
    # ["entry_point_BytesMakeOrChange { data_length: Some(32) }", 2426, 1.5, 0.099, 0.8, 0.1087],
    # ["entry_point_StepDst", 2388, 1.5, 0.1894, 2.0096, 0.2178],
    # ["entry_point_Loop { loop_count: Some(100000), loop_type: NoOp }", 27, 1.5, 2400.0128, 0, 0],
    # ["entry_point_Loop { loop_count: Some(10000), loop_type: Arithmetic }", 44, 1.5, 1312.02, 0, 0],
    # ["entry_point_CreateObjects { num_objects: 10, object_payload_size: 0 }", 666, 1.5, 4.8356, 8, 1.227],
    # ["entry_point_CreateObjects { num_objects: 10, object_payload_size: 10240 }", 103, 1.5, 505.4636, 16, 11.527],
    # ["entry_point_CreateObjects { num_objects: 100, object_payload_size: 0 }", 93, 1.5, 47.3516, 80, 12.27],
    # ["entry_point_CreateObjects { num_objects: 100, object_payload_size: 10240 }", 43, 1.5, 629.9516, 160, 115.27],
    # ["entry_point_InitializeVectorPicture { length: 40 }", 1605, 1.5, 2.6054, 2.4, 0.2531],
    # ["entry_point_VectorPicture { length: 40 }", 2850, 1.5, 0.1048, 2.4192, 0.1405],
    # ["entry_point_VectorPictureRead { length: 40 }", 2900, 1.5, 0.147, 2.4192, 0],
    # ["entry_point_InitializeVectorPicture { length: 30720 }", 30, 1.5, 1438.4294, 2.4, 9.4575],
    # ["entry_point_VectorPicture { length: 30720 }", 169, 1.5, 0.1048, 20.8512, 9.3449],
    # ["entry_point_VectorPictureRead { length: 30720 }", 189, 1.5, 0.147, 20.8512, 0],
    # ["entry_point_SmartTablePicture { length: 30720, num_points_per_txn: 200 }", 22, 4.254, 1623.39, 2.4192, 0.8106],
    # ["entry_point_SmartTablePicture { length: 1048576, num_points_per_txn: 1024 }", 3, 19.086, 11957.2104, 2.4192, 3.3392],
    # ["entry_point_TokenV1MintAndTransferFT", 1351, 1.5, 19.18316, 6.4384, 0.8813],
    # ["entry_point_TokenV1MintAndTransferNFTSequential", 971, 1.5, 30.42634, 6.4384, 0.9953],
    # ["Transfer", 2032, 1.5, 2.249, 2.4192, 0.2482],
    # ["CreateAccount", 1583, 1.5, 3.1536, 1.6, 0.247],
    # ["PublishModule", 138, 27.734, 2.86256, 37.6064, 1.6832],


    # new gas model:
    # ["entry_point_Nop", 4103, 1.5, 0.0012, 0, 0],
    # ["entry_point_BytesMakeOrChange { data_length: Some(32) }", 3411, 1.5, 0.099, 0.8, 0.1087],
    # ["entry_point_StepDst", 3270, 1.5, 0.1894, 2.0096, 0.2178],
    # ["entry_point_Loop { loop_count: Some(100000), loop_type: NoOp }", 28, 1.5, 2400.0128, 0, 0],
    # ["entry_point_Loop { loop_count: Some(10000), loop_type: Arithmetic }", 42, 1.5, 1312.02, 0, 0],
    # ["entry_point_CreateObjects { num_objects: 10, object_payload_size: 0 }", 1031, 1.5, 4.8356, 8, 1.227],
    # ["entry_point_CreateObjects { num_objects: 10, object_payload_size: 10240 }", 108, 1.5, 505.4636, 16, 11.527],
    # ["entry_point_CreateObjects { num_objects: 100, object_payload_size: 0 }", 148, 1.5, 47.3516, 80, 12.27],
    # ["entry_point_CreateObjects { num_objects: 100, object_payload_size: 10240 }", 50, 1.5, 629.9516, 160, 115.27],
    # ["entry_point_InitializeVectorPicture { length: 40 }", 2100, 1.5, 2.6054, 2.4, 0.2531],
    # ["entry_point_VectorPicture { length: 40 }", 3400, 1.5, 0.1048, 2.4192, 0.1405],
    # ["entry_point_VectorPictureRead { length: 40 }", 3480, 1.5, 0.147, 2.4192, 0],
    # ["entry_point_InitializeVectorPicture { length: 30720 }", 31, 1.5, 1438.4294, 2.4, 9.4575],
    # ["entry_point_VectorPicture { length: 30720 }", 180, 1.5, 0.1048, 20.8512, 9.3449],
    # ["entry_point_VectorPictureRead { length: 30720 }", 159, 1.5, 0.147, 20.8512, 0],
    # ["entry_point_SmartTablePicture { length: 30720, num_points_per_txn: 200 }", 17.8, 4.254, 1623.39, 2.4192, 0.8106],
    # ["entry_point_SmartTablePicture { length: 1048576, num_points_per_txn: 1024 }", 2.75, 19.086, 11957.2104, 2.4192, 3.3392],
    # ["entry_point_TokenV1MintAndTransferFT", 1719, 1.5, 19.18316, 6.4384, 0.8813],
    # ["entry_point_TokenV1MintAndTransferNFTSequential", 1150, 1.5, 30.42634, 6.4384, 0.9953],
    # ["Transfer", 2791, 1.5, 2.249, 2.4192, 0.2482],
    # ["CreateAccount", 2215, 1.5, 3.1536, 1.6, 0.247],
    # ["PublishModule", 148, 27.734, 2.86256, 37.6064, 1.6832],


    # original schedule:
    # ["entry_point_Nop", 4103, 1.5, 0.0012, 0, 0],
    # ["entry_point_BytesMakeOrChange { data_length: Some(32) }", 3411, 1.5, 0.099, 0.3, 0.474],
    # ["entry_point_StepDst", 3270, 1.5, 0.1894, 0.6024, 0.956],
    # ["entry_point_Loop { loop_count: Some(100000), loop_type: NoOp }", 28, 1.5, 2400.0128, 0, 0],
    # ["entry_point_Loop { loop_count: Some(10000), loop_type: Arithmetic }", 42, 1.5, 1448.024, 0, 0],
    # ["entry_point_CreateObjects { num_objects: 10, extra_size: 0 }", 1031, 1.5, 4.8356, 3, 7.54],
    # ["entry_point_CreateObjects { num_objects: 10, extra_size: 10240 }", 108, 1.5, 505.4636, 6, 213.54],
    # ["entry_point_CreateObjects { num_objects: 100, extra_size: 0 }", 148, 1.5, 47.3516, 30, 75.4],
    # ["entry_point_CreateObjects { num_objects: 100, extra_size: 10240 }", 50, 1.5, 629.9516, 60, 2135.4],
    # ["entry_point_InitializeVectorPicture { length: 40 }", 2100, 1.5, 2.6054, 0.9, 1.662],
    # ["entry_point_VectorPicture { length: 40 }", 3400, 1.5, 0.1048, 0.7422, 1.11],
    # ["entry_point_VectorPictureRead { length: 40 }", 3480, 1.5, 0.147, 0.7422, 0],
    # ["entry_point_InitializeVectorPicture { length: 30720 }", 31, 1.5, 1438.4294, 0.9, 185.75],
    # ["entry_point_VectorPicture { length: 30720 }", 180, 1.5, 0.1048, 55.968, 185.198],
    # ["entry_point_VectorPictureRead { length: 30720 }", 205, 1.5, 0.147, 55.968, 0],
    # ["entry_point_SmartTablePicture { length: 30720, num_points_per_txn: 200 }", 17.8, 4.254, 1620.5714, 0.7107, 7.712],
    # ["entry_point_SmartTablePicture { length: 1048576, num_points_per_txn: 1024 }", 2.75, 19.086, 11947.3918, 0.7107, 34.484],
    # ["entry_point_TokenV1MintAndTransferFT", 1719, 1.5, 17.30746, 2.0007, 5.726],
    # ["entry_point_TokenV1MintAndTransferNFTSequential", 1150, 1.5, 28.03864, 2.0007, 6.306],
    # ["Transfer", 2791, 1.5, 2.249, 0.663, 1.564],
    # ["CreateAccount", 2215, 1.5, 3.1536, 0.6, 1.54],
    # ["PublishModule", 148, 27.786, 2.8636, 99.9063, 28.616],
]

A = np.array([[row[i] * row[1] for i in range(2, 6)] for row in data])
b = np.array([[12000]]*len(data))

print("LSQ")
x = la.lstsq(A, b)[0]
print(np.matmul(A, x)) 
print(x)
print()

print("LSQ - Constrained")
res = sp.optimize.lsq_linear(A, np.matrix.flatten(b), (0.1, 10))
x = np.array([res.x]).transpose()
print(np.matmul(A, x)) 
print(x)
print((res.cost, res.optimality, res.message))

b_current = np.matmul(A, np.array([[1], [1], [1], [1]]))
b_lsq_constrained = np.matmul(A, x)
b_diff = b_lsq_constrained - b_current

import math

n = len(data)
table = [["name", "before gas/s", "optimized gas/s", "diff gas/s", "before gas/txn", "after gas/txn"]]
for i in range(n):
    row = []
    row.append(data[i][0])
    row.append("{}".format(math.ceil(b_current[i][0])))
    row.append("{}".format(math.ceil(b_lsq_constrained[i][0])))
    row.append(("{}" if b_diff[i][0] <= 0 else "+{}").format(math.ceil(b_diff[i][0])))
    row.append("{}".format(math.ceil(b_current[i][0] / data[i][1])))
    row.append("{}".format(math.ceil(b_lsq_constrained[i][0] / data[i][1])))
    table.append(row)

def tabularize(table):
    n = len(table)
    m = len(table[0])

    w = [0] * m

    for j in range(m):
        for i in range(n):
            w[j] = max(w[j], len(table[i][j]))

    for i in range(n):
        for j in range(m):
            k =  w[j] - len(table[i][j]) + 2 if j + 1 < n else 0
            print(table[i][j] + ' '*k, end = '')
        print()

tabularize(table)

max_before = math.ceil(max([b_current[i][0] for i in range(n)]))
min_before = math.ceil(min([b_current[i][0] for i in range(n)]))
max_after = math.ceil(max([b_lsq_constrained[i][0] for i in range(n)]))
min_after = math.ceil(min([b_lsq_constrained[i][0] for i in range(n)]))

print(f"{max_before=}, {min_before=}, ratio: {max_before/min_before}")
print(f"{max_after=}, {min_after=}, ratio: {max_after/min_after}")
