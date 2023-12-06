import numpy as np
import numpy.linalg as la
import scipy as sp

data = [
    ["entry_point_Nop", 3769, 1.5, 0.0012, 0, 0],
    ["entry_point_BytesMakeOrChange { data_length: Some(32) }", 3005, 1.5, 0.099, 0.3, 0.3],
    ["entry_point_StepDst", 2900, 1.5, 0.1894, 0.6024, 0.6],
    ["entry_point_InitializeVectorPicture { length: 40 }", 1809, 1.5, 2.9, 1.2018, 0.6],
    ["entry_point_VectorPicture { length: 40 }", 2721, 1.5, 0.1048, 0.7422, 0.3],
    ["entry_point_VectorPictureRead { length: 40 }", 2549, 1.5, 0.147, 0.7422, 0],
    ["entry_point_InitializeVectorPicture { length: 30720 }", 32, 1.5, 1438.724, 1.2018, 457.725],
    ["entry_point_VectorPicture { length: 30720 }", 181, 1.5, 0.1048, 55.968, 457.425],
    ["entry_point_VectorPictureRead { length: 30720 }", 200, 1.5, 0.147, 55.968, 0],
    ["entry_point_SmartTablePicture { length: 30720, num_points_per_txn: 200 }", 18.9, 4.254, 1620.5714, 0.7107, 1.5],
    ["entry_point_SmartTablePicture { length: 1048576, num_points_per_txn: 1024 }", 3.68, 19.086, 11947.3918, 0.7107, 5.79],
    ["entry_point_TokenV1MintAndTransferFT", 1555, 1.5, 17.30746, 2.0007, 2.1],
    ["entry_point_TokenV1MintAndTransferNFTSequential", 1111, 1.5, 28.03864, 2.0007, 2.4],
    ["Transfer", 2549, 1.5, 2.249, 0.663, 0.6],
    ["CreateAccount", 2122, 1.5, 3.1536, 0.6, 0.6],
]

A = np.array([[row[i] * row[1] for i in range(2, 6)] for row in data])
b = np.array([[20000]]*len(data))

print("LSQ")
x = la.lstsq(A, b)[0]
print(x)
print(np.matmul(A, x)) 
print()

print("LSQ - Constrained")
res = sp.optimize.lsq_linear(A, np.matrix.flatten(b), (0.25, 10))
x = np.array([res.x]).transpose()
print(np.matmul(A, x)) 
print(x)