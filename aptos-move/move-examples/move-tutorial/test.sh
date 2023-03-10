#!/bin/sh

APTOS=aptos

COMPILED="\
  step_1/BasicCoin\
  step_2/BasicCoin\
  step_2_sol/BasicCoin\
  step_4/BasicCoin\
  step_5/BasicCoin\
  step_5_sol/BasicCoin\
  step_6/BasicCoin\
  step_7/BasicCoin\
  step_8/BasicCoin\
  step_8_sol/BasicCoin\
"

TESTED="\
  step_2/BasicCoin\
  step_2_sol/BasicCoin\
  step_4/BasicCoin\
  step_5/BasicCoin\
  step_5_sol/BasicCoin\
  step_6/BasicCoin\
  step_7/BasicCoin\
  step_8/BasicCoin\
  step_8_sol/BasicCoin\
"


for compiled in $COMPILED
do
  (
    cd $compiled
    $APTOS move compile
  )
done

for tested in $TESTED
do
  (
    cd $tested
    $APTOS move test
  )
done
