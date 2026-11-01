"""Owned APFS image fixture; no supplied disk, mount or library paths."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import plistlib
import subprocess
import sys
import uuid


def command(*arguments):
    return subprocess.run(arguments, check=True, capture_output=True).stdout


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--probe", required=True)
    parser.add_argument("--preflight", action="store_true")
    args = parser.parse_args()
    if sys.platform != "darwin":
        raise RuntimeError("The APFS fixture requires a native macOS host.")
    workspace = Path.cwd().resolve()
    run_id = str(uuid.uuid4())
    directory = workspace / ".context" / "cross-volume" / run_id
    directory.mkdir(parents=True, exist_ok=False)
    if not directory.resolve().is_relative_to(workspace / ".context" / "cross-volume"):
        raise RuntimeError("The evidence directory escaped the task workspace.")
    image = directory / "destination.sparseimage"
    mount = directory / "mounted"
    report = {"version": 1, "runId": run_id, "platform": "darwin", "status": "not_run", "imageRemoved": False}
    device = None
    try:
        report["osVersion"] = command("/usr/bin/sw_vers").decode()
        with Path("/usr/bin/hdiutil").open("rb") as helper:
            report["hdiutilSha256"] = hashlib.file_digest(helper, "sha256").hexdigest()
        if args.preflight:
            report["status"] = "preflight_available"
            return
        probe = Path(args.probe).resolve(strict=True)
        if not probe.is_file() or not probe.is_relative_to(workspace / "target"):
            raise RuntimeError("Use the built task-workspace fixture probe.")
        if image.exists() or image.parent.resolve() != directory.resolve():
            raise RuntimeError("The image must be a new file in the owned evidence directory.")
        (directory / "hdiutil-create-help.txt").write_bytes(command("/usr/bin/hdiutil", "create", "-help"))
        command("/usr/bin/hdiutil", "create", "-quiet", "-size", "512m", "-type", "SPARSE", "-fs", "APFS", "-volname", "SH-" + run_id.replace("-", "")[:24], str(image))
        mount.mkdir()
        attached = plistlib.loads(command("/usr/bin/hdiutil", "attach", "-nobrowse", "-mountpoint", str(mount), "-plist", str(image)))
        entities = attached["system-entities"]
        roots = [entry["dev-entry"] for entry in entities if entry.get("dev-entry", "").startswith("/dev/disk") and entry["dev-entry"][len("/dev/disk"):].isdigit()]
        if not roots:
            raise RuntimeError("The created image did not return an owned device.")
        device = roots[0]
        if not any(entry.get("mount-point") == str(mount) for entry in entities):
            raise RuntimeError("The image is not mounted at the owned fixture mountpoint.")
        report["attachment"] = attached
        parent = mount / ("stillharbour-cross-volume-" + run_id)
        parent.mkdir(exist_ok=False)
        (parent / ".stillharbour-fixture-owner").write_text(run_id, encoding="utf-8")
        (directory / "scope.json").write_text(json.dumps({"version": 1, "runId": run_id, "image": str(image), "destinationParent": str(parent)}), encoding="utf-8")
        environment = {**os.environ, "STILLHARBOUR_CROSS_VOLUME_RUN": run_id}
        completed = subprocess.run([str(probe)], env=environment, capture_output=True, check=False)
        (directory / "probe.stdout").write_bytes(completed.stdout)
        (directory / "probe.stderr").write_bytes(completed.stderr)
        completed.check_returncode()
        receipt = json.loads(completed.stdout)
        if not receipt["distinctVolumes"] or not receipt["sourcesRetained"] or not receipt["destinationOnlyRetained"]:
            raise RuntimeError("The receipt did not prove native cross-volume copying.")
        report["receipt"] = receipt
        report["status"] = "passed"
    except Exception as error:
        report["status"] = "failed"
        report["error"] = str(error)
        raise
    finally:
        try:
            if image.exists():
                images = plistlib.loads(command("/usr/bin/hdiutil", "info", "-plist"))["images"]
                owned = [entry for entry in images if Path(entry.get("image-path", "")).resolve() == image.resolve()]
                if owned and not device:
                    candidates = [entry["dev-entry"] for entry in owned[0]["system-entities"] if entry.get("dev-entry", "").startswith("/dev/disk") and entry["dev-entry"][len("/dev/disk"):].isdigit()]
                    device = candidates[0] if candidates else None
                if owned and (len(owned) != 1 or not device or not any(entry.get("dev-entry") == device for entry in owned[0]["system-entities"])):
                    raise RuntimeError("The device no longer belongs to the created image; no detach attempted.")
                if owned:
                    command("/usr/bin/hdiutil", "detach", device)
            if report["status"] == "passed":
                if image.is_symlink() or image.parent.resolve() != directory.resolve() or image.name != "destination.sparseimage" or not image.is_file():
                    raise RuntimeError("The image escaped its exact owned file path; no removal attempted.")
                image.unlink()
                report["imageRemoved"] = True
        except Exception as error:
            report["status"] = "cleanup_failed"
            report["cleanupError"] = str(error)
            raise
        finally:
            (directory / "report.json").write_text(json.dumps(report, indent=2), encoding="utf-8")


if __name__ == "__main__":
    main()
