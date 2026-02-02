#!/usr/bin/env python3
"""
SENTINEL Python SDK - Basic Usage Example

This example demonstrates how to use the SENTINEL SDK to:
- Create a goal manifold
- Add goals with success criteria
- Track progress
- Validate alignment
"""

import asyncio
import os
from datetime import datetime

from sentinel_sdk import (
    SentinelClient,
    GoalStatus,
    ProbabilityDistribution,
    DistributionType,
    GoalMetadata,
)


# Configuration
SENTINEL_URL = os.environ.get("SENTINEL_URL", "http://localhost:8080")


async def main():
    """Run the example."""
    # Initialize client with context manager for automatic cleanup
    async with SentinelClient(SENTINEL_URL, timeout=30.0) as client:
        print(f"🔗 Connected to SENTINEL at {SENTINEL_URL}")
        
        # Step 1: Create a new goal manifold for a data pipeline project
        manifold = await client.create_goal_manifold(
            description="Build a real-time data processing pipeline for analytics",
            constraints=[
                "Use Python 3.11+",
                "Apache Kafka for streaming",
                "PostgreSQL for storage",
                "Docker for deployment",
                "Test coverage > 85%",
            ],
            languages=["Python", "SQL"],
            frameworks=["FastAPI", "Apache Kafka", "SQLAlchemy"],
            target_platform="cloud",
        )
        print(f"✅ Created goal manifold: {manifold.id}")
        print(f"   Root intent: {manifold.root_intent.description}")
        
        # Step 2: Add infrastructure setup goal
        setup_goal = await client.add_goal(
            manifold_id=manifold.id,
            description="Setup Docker infrastructure and Kafka cluster",
            success_criteria=[
                {"type": "file_exists", "params": {"path": "docker-compose.yml"}},
                {"type": "file_exists", "params": {"path": "Dockerfile"}},
                {"type": "api_available", "params": {"endpoint": "http://localhost:9092", "method": "GET"}},
            ],
            complexity_estimate=ProbabilityDistribution(
                mean=5.0,
                std_dev=1.0,
                min=3.0,
                max=7.0,
                distribution_type=DistributionType.NORMAL,
            ),
            value_to_root=0.15,
            status=GoalStatus.READY,
            metadata=GoalMetadata(
                tags=["infrastructure", "docker", "kafka"],
                notes="Foundation for the data pipeline",
                priority=1.0,
            ),
        )
        print(f"✅ Added infrastructure goal: {setup_goal.id}")
        
        # Step 3: Add data ingestion service goal
        ingestion_goal = await client.add_goal(
            manifold_id=manifold.id,
            description="Implement Kafka consumer for data ingestion",
            success_criteria=[
                {"type": "tests_passing", "params": {"suite": "ingestion", "min_coverage": 0.85}},
                {"type": "file_exists", "params": {"path": "src/consumers/data_consumer.py"}},
            ],
            dependencies=[setup_goal.id],
            complexity_estimate=ProbabilityDistribution(
                mean=7.0,
                std_dev=1.5,
                min=5.0,
                max=9.0,
                distribution_type=DistributionType.NORMAL,
            ),
            value_to_root=0.25,
            status=GoalStatus.PENDING,
            validation_tests=["test/consumers/test_data_consumer.py"],
            metadata=GoalMetadata(
                tags=["kafka", "consumer", "ingestion"],
                notes="Critical for data pipeline input",
                priority=0.9,
            ),
        )
        print(f"✅ Added ingestion goal: {ingestion_goal.id}")
        
        # Step 4: Add data processing service goal
        processing_goal = await client.add_goal(
            manifold_id=manifold.id,
            description="Build real-time data transformation service",
            success_criteria=[
                {"type": "tests_passing", "params": {"suite": "processing", "min_coverage": 0.9}},
                {"type": "api_available", "params": {"endpoint": "/api/v1/process", "method": "POST"}},
            ],
            dependencies=[setup_goal.id],
            complexity_estimate=ProbabilityDistribution(
                mean=8.0,
                std_dev=1.0,
                min=6.0,
                max=10.0,
                distribution_type=DistributionType.NORMAL,
            ),
            value_to_root=0.3,
            status=GoalStatus.PENDING,
            validation_tests=["test/processing/test_transformer.py"],
            metadata=GoalMetadata(
                tags=["processing", "transformation", "real-time"],
                notes="Core business logic",
                priority=1.0,
            ),
        )
        print(f"✅ Added processing goal: {processing_goal.id}")
        
        # Step 5: Add analytics API goal (depends on processing)
        api_goal = await client.add_goal(
            manifold_id=manifold.id,
            description="Create REST API for analytics queries",
            success_criteria=[
                {"type": "tests_passing", "params": {"suite": "api", "min_coverage": 0.8}},
                {"type": "api_available", "params": {"endpoint": "/api/v1/analytics", "method": "GET"}},
                {"type": "api_available", "params": {"endpoint": "/api/v1/metrics", "method": "GET"}},
            ],
            dependencies=[processing_goal.id],
            complexity_estimate=ProbabilityDistribution(
                mean=6.0,
                std_dev=1.0,
                min=4.0,
                max=8.0,
                distribution_type=DistributionType.NORMAL,
            ),
            value_to_root=0.2,
            status=GoalStatus.PENDING,
            validation_tests=["test/api/test_analytics.py"],
            metadata=GoalMetadata(
                tags=["api", "rest", "analytics"],
                notes="User-facing API",
                priority=0.8,
            ),
        )
        print(f"✅ Added API goal: {api_goal.id}")
        
        # Step 6: Mark setup goal as completed
        completed_setup = await client.update_goal_status(
            manifold_id=manifold.id,
            goal_id=setup_goal.id,
            status=GoalStatus.COMPLETED,
        )
        print(f"✅ Marked setup goal as completed")
        
        # Step 7: Check overall progress
        updated_manifold = await client.get_goal_manifold(manifold.id)
        print(f"\n📊 Project Progress:")
        print(f"   Completion: {updated_manifold.completion_percentage * 100:.1f}%")
        print(f"   Total goals: {len(updated_manifold.goals)}")
        
        # Step 8: Validate alignment
        alignment = await client.get_alignment_score(manifold.id)
        print(f"\n🎯 Alignment Score: {alignment.score:.1f}/100")
        print(f"   Confidence: {alignment.confidence * 100:.1f}%")
        
        if alignment.violations:
            print(f"   ⚠️  Violations found: {len(alignment.violations)}")
            for v in alignment.violations:
                print(f"      - {v.description} ({v.severity.value})")
        else:
            print(f"   ✅ No alignment violations")
        
        # Step 9: Validate invariants
        validation = await client.validate_invariants(manifold.id)
        print(f"\n🔒 Invariant Validation:")
        print(f"   Passed: {validation.passed}")
        if validation.violations:
            print(f"   ⚠️  Violations: {len(validation.violations)}")
        
        # Step 10: List all manifolds
        manifolds = await client.list_goal_manifolds()
        print(f"\n📋 Total manifolds in system: {len(manifolds)}")
        
        print("\n👋 Done!")


async def subscribe_example():
    """Example of subscribing to real-time updates."""
    async with SentinelClient(SENTINEL_URL) as client:
        # First create a manifold
        manifold = await client.create_goal_manifold(
            description="Test subscription",
            constraints=["Test"],
        )
        
        print(f"🔄 Subscribing to updates for manifold: {manifold.id}")
        
        # Subscribe to updates with timeout
        try:
            async for update in client.subscribe_to_updates(manifold.id):
                print(f"📡 Update: {update.type} at {update.timestamp}")
                
                # Exit after first update for demo
                break
                
        except asyncio.TimeoutError:
            print("⏱️  Subscription timed out (expected in demo)")


if __name__ == "__main__":
    # Run main example
    asyncio.run(main())
    
    # Uncomment to run subscription example
    # asyncio.run(subscribe_example())
