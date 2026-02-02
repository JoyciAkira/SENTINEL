from typing import List, Optional, Dict, Any, Union
from enum import Enum
from datetime import datetime
from uuid import UUID
from pydantic import BaseModel, Field, root_validator

class InvariantSeverity(str, Enum):
    WARNING = "Warning"
    ERROR = "Error"
    CRITICAL = "Critical"
    # Fallback for lowercase
    WARNING_LOWER = "warning"
    ERROR_LOWER = "error"
    CRITICAL_LOWER = "critical"

class Predicate(BaseModel):
    # Rust serializes complex enums as { "VariantName": { ...body... } } or just "VariantName"
    # To handle this dynamically in Pydantic v2 is complex without custom validators.
    # For v1 SDK, we treat it as a flexible dictionary container.
    content: Optional[Union[str, Dict[str, Any]]] = None

    @root_validator(pre=True)
    def parse_rust_enum(cls, values):
        # This catches the Rust enum serialization format and stores it safely
        return {"content": values}

class Invariant(BaseModel):
    id: UUID
    description: str
    severity: InvariantSeverity
    predicate: Any # Simplified for now

class Intent(BaseModel):
    description: str
    constraints: List[str] = Field(default_factory=list)
    expected_outcomes: List[str] = Field(default_factory=list)
    target_platform: Optional[str] = None
    languages: List[str] = Field(default_factory=list)
    frameworks: List[str] = Field(default_factory=list)
    infrastructure_map: Dict[str, str] = Field(default_factory=dict)

class GoalStatus(str, Enum):
    # Rust Serde defaults often to the variant name, but can be configured to lowercase
    # We add both to be safe during the transition
    PENDING = "Pending"
    READY = "Ready"
    IN_PROGRESS = "InProgress"
    VALIDATING = "Validating"
    COMPLETED = "Completed"
    BLOCKED = "Blocked"
    FAILED = "Failed"
    DEPRECATED = "Deprecated"
    
    # Lowercase mappings observed in error logs
    pending = "pending"
    ready = "ready"
    in_progress = "inprogress"
    validating = "validating"
    completed = "completed"
    blocked = "blocked"
    failed = "failed"
    deprecated = "deprecated"

class Goal(BaseModel):
    id: UUID
    description: str
    parent_id: Optional[UUID] = None
    dependencies: List[UUID] = Field(default_factory=list)
    status: GoalStatus = GoalStatus.pending
    # Success criteria come as complex Rust enums
    success_criteria: List[Any] = Field(default_factory=list)
    value_to_root: float = 0.0
    
class ManifoldVersion(BaseModel):
    version: int
    timestamp: datetime
    hash: str
    change_description: str

class GoalManifold(BaseModel):
    root_intent: Intent
    sensitivity: float
    goals: List[Goal] = Field(default_factory=list)
    
    # These fields are present in the core Rust struct but might be omitted 
    # in the simplified CLI JSON output. We verify logic based on what we receive.
    invariants: List[Invariant] = Field(default_factory=list)
    created_at: Optional[datetime] = None
    updated_at: Optional[datetime] = None
    integrity_hash: Optional[str] = None
    version_history: List[ManifoldVersion] = Field(default_factory=list)