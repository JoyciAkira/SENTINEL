import subprocess
import json
import os
from typing import Optional, List
from pathlib import Path
from .types import GoalManifold, Intent

class SentinelError(Exception):
    pass

class SentinelClient:
    """
    Synchronous client for the Sentinel CLI.
    Wraps CLI commands to provide a Pythonic API.
    """
    
    def __init__(self, executable: str = "sentinel", working_dir: Optional[str] = None):
        self.executable = executable
        self.working_dir = working_dir or os.getcwd()

    def _run_command(self, args: List[str]) -> str:
        """Executes a sentinel command and returns stdout."""
        cmd = [self.executable] + args
        try:
            result = subprocess.run(
                cmd,
                cwd=self.working_dir,
                capture_output=True,
                text=True,
                check=True
            )
            return result.stdout
        except subprocess.CalledProcessError as e:
            raise SentinelError(f"Sentinel command failed: {e.stderr}") from e
        except FileNotFoundError:
            raise SentinelError(f"Sentinel executable '{self.executable}' not found. Is it installed?")

    def init(self, description: str) -> None:
        """Initialize a new Sentinel project."""
        self._run_command(["init", description])

    def status(self) -> GoalManifold:
        """
        Get the current status of the Goal Manifold.
        Returns a typed GoalManifold object.
        """
        output = self._run_command(["status", "--json"])
        try:
            data = json.loads(output)
            # The CLI returns a wrapper object status_report, we need to extract the manifold
            # Based on crates/sentinel-cli/src/main.rs: Status -> json output structure
            if "manifold" in data:
                # We need to adapt the raw JSON to match our Pydantic model
                # The CLI structure might be slightly different from the internal Rust struct
                # so we map what we can.
                m_data = data["manifold"]
                
                # Transform goal_dag to list of goals for the SDK
                goals_list = []
                if "goal_dag" in m_data and "nodes" in m_data["goal_dag"]:
                    goals_list = m_data["goal_dag"]["nodes"]
                
                m_data["goals"] = goals_list
                
                return GoalManifold(**m_data)
            else:
                raise SentinelError("Invalid status response format")
        except json.JSONDecodeError as e:
            raise SentinelError(f"Failed to parse Sentinel output: {e}")

    def verify(self, sandbox: bool = True) -> bool:
        """
        Run verification suite.
        Returns True if verification passes.
        """
        args = ["verify"]
        if not sandbox:
            args.append("--sandbox=false")
            
        try:
            self._run_command(args)
            return True
        except SentinelError:
            return False

    def decompose(self, goal_id: str) -> None:
        """Decompose a complex goal into sub-tasks."""
        self._run_command(["decompose", goal_id])
