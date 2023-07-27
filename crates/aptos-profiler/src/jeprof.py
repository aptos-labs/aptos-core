import subprocess
import sys

def execute_command(command):
    try:
        output = subprocess.check_output(command, shell=True, stderr=subprocess.STDOUT)
        return output.decode('utf-8').strip()
    except subprocess.CalledProcessError as e:
        return f"Command execution failed with error code {e.returncode}. Output:\n{e.output.decode('utf-8').strip()}"

arg1 = sys.argv[1]


command = "jeprof --show_bytes /home/yunusozer/aptos-core/target/release/aptos-node /home/yunusozer/aptos-core/prof.*.heap --svg  > /home/yunusozer/aptos-core/crates/aptos-inspection-service/src/server/memory_profile/heap.svg"
result = execute_command(command)
command = "jeprof --show_bytes /home/yunusozer/aptos-core/target/release/aptos-node /home/yunusozer/aptos-core/prof.*.heap --text  > /home/yunusozer/aptos-core/crates/aptos-inspection-service/src/server/memory_profile/heap.txt"
result = execute_command(command)


#command = "rm /home/ubuntu/aptos-core/crates/aptos-inspection-service/src/server/memory_profile/prof*"
#result = execute_command(command)