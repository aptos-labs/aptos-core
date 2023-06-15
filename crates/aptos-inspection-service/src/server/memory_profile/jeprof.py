import subprocess

def execute_command(command):
    try:
        output = subprocess.check_output(command, shell=True, stderr=subprocess.STDOUT)
        return output.decode('utf-8').strip()
    except subprocess.CalledProcessError as e:
        return f"Command execution failed with error code {e.returncode}. Output:\n{e.output.decode('utf-8').strip()}"

command = "jeprof --show_bytes /home/ubuntu/aptos-core/target/release/aptos-node /home/ubuntu/aptos-core/crates/aptos-inspection-service/src/server/memory_profile/prof.*.heap --svg  > /home/ubuntu/aptos-core/crates/aptos-inspection-service/src/server/memory_profile/heap.svg"
result = execute_command(command)
print(result)
command = "rm /home/ubuntu/aptos-core/crates/aptos-inspection-service/src/server/memory_profile/prof*"
result = execute_command(command)