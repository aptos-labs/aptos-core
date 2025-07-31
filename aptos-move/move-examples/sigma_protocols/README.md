# Notes

Due to Move's limitations (e.g., cyclic dependency issues; cannot have the same function named `length` for different
structs; lambdas can only be used with `inline` functions, etc.), I had to split up the implementation across several files. 