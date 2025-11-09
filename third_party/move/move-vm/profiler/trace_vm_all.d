#pragma D option quiet

inline string FN_IN = "fn  ->";
inline string FN_OUT = "fn  <-";
inline string INSTR_IN = "instr ->";
inline string INSTR_OUT = "instr <-";

/* Function probes */

vm_profiler*:::function_entry
{
  printf("%s %s\n", FN_IN, copyinstr(arg0));
}

vm_profiler*:::function_exit
{
  printf("%s %s [dt=%llu ns]\n", FN_OUT, copyinstr(arg0), (unsigned long long)arg1);
}

/* Instruction probes */

vm_profiler*:::instruction_entry
{
  printf("%s %s\n", INSTR_IN, copyinstr(arg0));
}

vm_profiler*:::instruction_exit
{
  printf("%s %s [dt=%llu ns]\n", INSTR_OUT, copyinstr(arg0), (unsigned long long)arg1);
}
