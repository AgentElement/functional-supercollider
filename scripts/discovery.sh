#!/bin/bash

#SBATCH -N 1            # number of nodes
#SBATCH -c 128          # number of cores
#SBATCH -t 0-12:00:00   # time in d-hh:mm:ss
#SBATCH -p general      # partition
#SBATCH -q public       # QOS
#SBATCH -o slurm.%j.out # file to save job's STDOUT (%j = JobId)
#SBATCH -e slurm.%j.err # file to save job's STDERR (%j = JobId)
#SBATCH --mail-type=ALL # Send an e-mail when a job starts, stops, or fails
#SBATCH --mail-user="%u@asu.edu"
#SBATCH --export=NONE   # Purge the job-submitting shell environment

module load rust/1.82.0
cd ~/cwd/functional-supercollider
cargo run --release -- --experiment measure-initial-population,
cargo run --release -- --experiment add-scc-population-from-random-inputs,
cargo run --release -- --experiment add-scc-population-from-ski-inputs,
cargo run --release -- --experiment add-scc-population-from-skip-inputs,
cargo run --release -- --experiment scc-population-from-random-inputs-with-tests,
cargo run --release -- --experiment add-population-from-random-inputs-with-tests,
cargo run --release -- --experiment add-population-from-random-inputs-with-add-succ-tests,
cargo run --release -- --experiment scc-population-from-ski-inputs-with-tests,
cargo run --release -- --experiment add-population-from-ski-inputs-with-tests,
cargo run --release -- --experiment add-population-from-ski-inputs-with-add-succ-tests,
