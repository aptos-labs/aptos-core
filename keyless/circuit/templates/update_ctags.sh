#!/bin/bash

find . -name "*.circom" | ctags --language-force=C -L-
