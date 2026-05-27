#pragma D

/* Function probes */

vm_profiler*:::function_entry
{
  printf("BEGIN %s\n", copyinstr(arg0));
}

vm_profiler*:::function_exit
{
  printf("END %llu\n", (unsigned long long)arg0);
}
