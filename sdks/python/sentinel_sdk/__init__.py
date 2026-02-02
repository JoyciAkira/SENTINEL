"""
SENTINEL Python SDK

This SDK provides a type-safe interface to interact with the SENTINEL
goal-aligned AI coding agent system.

Example:
    >>> from sentinel_sdk import SentinelClient
    >>> 
    >>> client = SentinelClient("http://localhost:8080")
    >>> 
    >>> # Create a new goal manifold
    >>> manifold = await client.create_goal_manifold(
    ...     description="Build a REST API",
    ...     constraints=["Use Python", "FastAPI"]
    ... )
    >>> 
    >>> # Add goals
    >>> goal = await client.add_goal(
    ...     manifold_id=manifold.id,
    ...     description="Implement authentication",
    ...     success_criteria=[{"type": "test_passing", "suite": "auth"}]
    ... )
"""

from typing import Optional, List, Dict, Any, Callable, AsyncIterator
from dataclasses import dataclass, field
from datetime import datetime
from enum import Enum
import asyncio
import json

import httpx
import websockets
from pydantic import BaseModel, Field


class GoalStatus(str, Enum):
    """Goal execution status."""
    PENDING = "pending"
    READY = "ready"
    IN_PROGRESS = "in_progress"
    VALIDATING = "validating"
    COMPLETED = "completed"
    BLOCKED = "blocked"
    FAILED = "failed"
    DEPRECATED = "deprecated"


class DistributionType(str, Enum):
    """Type of probability distribution."""
    NORMAL = "normal"
    UNIFORM = "uniform"
    POINT = "point"


class InvariantSeverity(str, Enum):
    """Severity of invariant violation."""
    WARNING = "warning"
    ERROR = "error"
    CRITICAL = "critical"


class ProbabilityDistribution(BaseModel):
    """Probability distribution for metrics."""
    mean: float
    std_dev: float
    min: float
    max: float
    distribution_type: DistributionType = Field(default=DistributionType.NORMAL)


class Intent(BaseModel):
    """The original user intent."""
    description: str
    constraints: List[str]
    expected_outcomes: List[str] = Field(default_factory=list)
    target_platform: Optional[str] = None
    languages: List[str] = Field(default_factory=list)
    frameworks: List[str] = Field(default_factory=list)


class SuccessCriterion(BaseModel):
    """A single success criterion."""
    type: str
    params: Dict[str, Any] = Field(default_factory=dict)


class GoalMetadata(BaseModel):
    """Metadata for a goal."""
    tags: List[str] = Field(default_factory=list)
    notes: str = ""
    priority: float = 0.5


class Goal(BaseModel):
    """A single goal in the Goal Manifold."""
    id: str
    description: str
    success_criteria: List[SuccessCriterion]
    dependencies: List[str] = Field(default_factory=list)
    anti_dependencies: List[str] = Field(default_factory=list)
    complexity_estimate: ProbabilityDistribution
    value_to_root: float
    status: GoalStatus
    parent_id: Optional[str] = None
    validation_tests: List[str] = Field(default_factory=list)
    metadata: GoalMetadata = Field(default_factory=GoalMetadata)
    created_at: datetime
    updated_at: datetime


class Invariant(BaseModel):
    """A hard constraint that must never be violated."""
    id: str
    description: str
    severity: InvariantSeverity


class GoalManifold(BaseModel):
    """The immutable core of Sentinel."""
    id: str
    root_intent: Intent
    goals: List[Goal] = Field(default_factory=list)
    invariants: List[Invariant] = Field(default_factory=list)
    completion_percentage: float = 0.0
    created_at: datetime
    updated_at: datetime


class AlignmentViolation(BaseModel):
    """An alignment violation detected by Sentinel."""
    id: str
    description: str
    severity: InvariantSeverity


class AlignmentScore(BaseModel):
    """Result of an alignment evaluation."""
    score: float
    confidence: float
    violations: List[AlignmentViolation] = Field(default_factory=list)


class InvariantValidationResult(BaseModel):
    """Result of invariant validation."""
    passed: bool
    violations: List[InvariantViolation] = Field(default_factory=list)


class ManifoldUpdate(BaseModel):
    """A real-time update from the manifold."""
    type: str
    timestamp: datetime
    data: Dict[str, Any] = Field(default_factory=dict)


class SentinelError(Exception):
    """Error raised by the Sentinel SDK."""
    
    def __init__(self, message: str, status_code: Optional[int] = None):
        super().__init__(message)
        self.status_code = status_code


class SentinelClient:
    """
    Main client for interacting with SENTINEL.
    
    Args:
        base_url: Base URL of the SENTINEL daemon
        auth_token: Optional authentication token
        timeout: Request timeout in seconds
    
    Example:
        >>> client = SentinelClient("http://localhost:8080")
        >>> manifold = await client.create_goal_manifold(
        ...     description="Build API",
        ...     constraints=["Python", "FastAPI"]
        ... )
    """
    
    def __init__(
        self,
        base_url: str,
        auth_token: Optional[str] = None,
        timeout: float = 30.0
    ):
        self.base_url = base_url.rstrip('/')
        self.auth_token = auth_token
        self.timeout = timeout
        self._client: Optional[httpx.AsyncClient] = None
    
    async def __aenter__(self):
        await self.connect()
        return self
    
    async def __aexit__(self, exc_type, exc_val, exc_tb):
        await self.close()
    
    async def connect(self):
        """Initialize the HTTP client."""
        headers = {"Content-Type": "application/json"}
        if self.auth_token:
            headers["Authorization"] = f"Bearer {self.auth_token}"
        
        self._client = httpx.AsyncClient(
            base_url=self.base_url,
            headers=headers,
            timeout=self.timeout
        )
    
    async def close(self):
        """Close the HTTP client."""
        if self._client:
            await self._client.aclose()
            self._client = None
    
    def _ensure_client(self) -> httpx.AsyncClient:
        """Ensure the client is connected."""
        if not self._client:
            raise SentinelError("Client not connected. Use 'async with' or call connect() first.")
        return self._client
    
    async def _request(
        self,
        method: str,
        path: str,
        **kwargs
    ) -> Dict[str, Any]:
        """Make an HTTP request."""
        client = self._ensure_client()
        response = await client.request(method, f"/api/v1{path}", **kwargs)
        
        if response.status_code >= 400:
            raise SentinelError(
                f"HTTP {response.status_code}: {response.text}",
                response.status_code
            )
        
        return response.json()
    
    async def create_goal_manifold(
        self,
        description: str,
        constraints: List[str],
        **kwargs
    ) -> GoalManifold:
        """
        Create a new goal manifold.
        
        Args:
            description: Natural language description of the objective
            constraints: User-specified constraints
            **kwargs: Additional intent fields
        
        Returns:
            The created GoalManifold
        """
        intent = Intent(
            description=description,
            constraints=constraints,
            **kwargs
        )
        data = await self._request("POST", "/manifolds", json=intent.model_dump())
        return GoalManifold(**data)
    
    async def get_goal_manifold(self, manifold_id: str) -> GoalManifold:
        """Get a goal manifold by ID."""
        data = await self._request("GET", f"/manifolds/{manifold_id}")
        return GoalManifold(**data)
    
    async def list_goal_manifolds(self) -> List[GoalManifold]:
        """List all goal manifolds."""
        data = await self._request("GET", "/manifolds")
        return [GoalManifold(**m) for m in data]
    
    async def add_goal(
        self,
        manifold_id: str,
        description: str,
        success_criteria: List[Dict[str, Any]],
        **kwargs
    ) -> Goal:
        """
        Add a goal to a manifold.
        
        Args:
            manifold_id: ID of the manifold
            description: Goal description
            success_criteria: List of success criteria
            **kwargs: Additional goal fields
        
        Returns:
            The created Goal
        """
        goal_data = {
            "description": description,
            "success_criteria": success_criteria,
            **kwargs
        }
        data = await self._request(
            "POST",
            f"/manifolds/{manifold_id}/goals",
            json=goal_data
        )
        return Goal(**data)
    
    async def update_goal_status(
        self,
        manifold_id: str,
        goal_id: str,
        status: GoalStatus
    ) -> Goal:
        """Update the status of a goal."""
        data = await self._request(
            "PATCH",
            f"/manifolds/{manifold_id}/goals/{goal_id}/status",
            json={"status": status.value}
        )
        return Goal(**data)
    
    async def validate_invariants(self, manifold_id: str) -> InvariantValidationResult:
        """Validate all invariants in a manifold."""
        data = await self._request("GET", f"/manifolds/{manifold_id}/validate")
        return InvariantValidationResult(**data)
    
    async def get_alignment_score(self, manifold_id: str) -> AlignmentScore:
        """Get the alignment score for a manifold."""
        data = await self._request("GET", f"/manifolds/{manifold_id}/alignment")
        return AlignmentScore(**data)
    
    async def subscribe_to_updates(
        self,
        manifold_id: str
    ) -> AsyncIterator[ManifoldUpdate]:
        """
        Subscribe to real-time updates for a manifold.
        
        Yields:
            ManifoldUpdate objects as they occur
        """
        ws_url = self.base_url.replace("http", "ws")
        uri = f"{ws_url}/ws/manifolds/{manifold_id}"
        
        async with websockets.connect(uri) as websocket:
            async for message in websocket:
                data = json.loads(message)
                yield ManifoldUpdate(**data)


# Convenience exports
__all__ = [
    "SentinelClient",
    "SentinelError",
    "GoalManifold",
    "Goal",
    "GoalStatus",
    "Intent",
    "SuccessCriterion",
    "ProbabilityDistribution",
    "DistributionType",
    "Invariant",
    "InvariantSeverity",
    "InvariantValidationResult",
    "AlignmentScore",
    "AlignmentViolation",
    "ManifoldUpdate",
    "GoalMetadata",
]
