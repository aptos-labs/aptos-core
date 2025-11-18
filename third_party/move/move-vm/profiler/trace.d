#pragma D

/* Function probes */

vm_profiler*:::function_entry
{
  printf("BEGIN %s\n", copyinstr(arg0));
}

vm_profiler*:::function_exit
{
  printf("END %s %llu\n", copyinstr(arg0), (unsigned long long)arg1);
}
