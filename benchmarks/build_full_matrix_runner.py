#!/usr/bin/env python3
import base64
import os
import subprocess

def main():
    repo_root = os.path.abspath(os.path.join(os.path.dirname(__file__), ".."))
    tarball_path = "/tmp/k0maru_src_clean.tar.gz"
    
    print("[1] Building clean source tarball...")
    cmd = [
        "tar", "-czf", tarball_path,
        "-C", repo_root,
        "Cargo.toml", "Cargo.lock", "crates", "src"
    ]
    subprocess.run(cmd, check=True)
    
    file_size = os.path.getsize(tarball_path)
    print(f"Tarball created: {file_size / 1024:.1f} KB")
    
    with open(tarball_path, "rb") as f:
        encoded_src = base64.b64encode(f.read()).decode("ascii")
    
    print(f"Base64 string length: {len(encoded_src)} characters")

    template_path = os.path.join(repo_root, "benchmarks", "full_matrix_template.py")
    with open(template_path, "r", encoding="utf-8") as f:
        template_content = f.read()

    runner_content = template_content.replace("%%EMBEDDED_B64%%", encoded_src)
    
    runner_dest = os.path.join(repo_root, "benchmarks", "run_colab_full_matrix_benchmark.py")
    with open(runner_dest, "w", encoding="utf-8") as f:
        f.write(runner_content)
    
    print(f"[OK] Generated {runner_dest} ({os.path.getsize(runner_dest)/1024:.1f} KB)")

if __name__ == "__main__":
    main()
