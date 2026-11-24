from pathlib import Path
import csv
import subprocess

from datetime import datetime
from pathlib import Path

HEURISTICS = [
    "first-layer-optimized",
    "biggest-layer-optimized"
]

TIME_LIMITS_MS = [
    "1000",
    "2000",
] 

LEVEL_LIMITS = [
    "3",
    "13",
]

def parse_result(stdout_text: str)->list[int]:
    for line in stdout_text.splitlines():
        if line.startswith("RESULT;"):
            parts = line.split(";")
            nodes_before = int(parts[1])
            nodes_after = int(parts[2])
            diff = int(parts[3])
            sifted_layers = int(parts[4])
            return [nodes_before, nodes_after, diff, sifted_layers]
    return[-1]

#Basis-Ordner
base_dir = Path("../preprocessed-inputs/207234985/bayesian-networks/")

print(f"Basis-Ordner existiert?: {base_dir.exists()}")
print(f"Absoluter Pfad: {base_dir.resolve()}")

# print(list(base_dir.iterdir()))


# .nnf-Datein im Unterordner
test_dirs = [p for p in base_dir.glob("or-50-10*") if p.is_dir()]
print(f"Gefundene Ordner in or-50-10: {len(test_dirs)}")

test_files = [
    file for d in test_dirs 
    if d.is_dir()
    for file in d.rglob("*.nnf")
    if file.is_file()
    ]

print(f"found {len(test_files)} files:")

subset = test_files[:2]

binary_path = Path("target/release/oxidd-cli")
print(f" found binary and exists = {binary_path.exists()}")

timestamp = datetime.now().strftime("%Y-%m-%d_%H-%M")
filename = f"benchmark_results_{timestamp}.csv"
output_path = Path(f"benchmark_results/{filename}")
with open(output_path, mode="w", newline="", encoding="utf-8") as f:
    writer = csv.writer(f)

    writer.writerow([
        "file",
        "heuristic",
        "time_limit_ms",
        "level_limit",
        "nodes_before",
        "nodes_after",
        "difference",
        "sifted_levels",
    ])

    for file in subset:

        for heuristic in HEURISTICS:

            for time in TIME_LIMITS_MS:

                for level in LEVEL_LIMITS:


                    print(f"{file.name} exists = {file.exists()}")
                    cmd = [
                    "./target/release/oxidd-cli",
                    "--read-var-order",
                    "--parallel",
                    "--progress",
                    "--heuristic", heuristic,        
                    "--time-limit-ms", time,
                    "--level-limit", level,
                    file.as_posix()
                    ]

                    result = subprocess.run(
                        cmd,
                        capture_output=True,
                        text=True
                    )

                    if result.returncode == 0:
                        print("Erfolgreich! Output:")
                        for str in parse_result(result.stdout):
                            print(f"{str}")
                    else:
                        print(f"Fehler (Exit-Code {result.returncode})")

                    result_list = parse_result(result.stdout)
                    writer.writerow([file.name, heuristic, time, level, result_list[0], result_list[1], result_list[2], result_list[3]])

                    f.flush()

print("Benchmark abgeschlossen und gespeichert!")