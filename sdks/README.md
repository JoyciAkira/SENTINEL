# SENTINEL SDKs

Official SDKs for interacting with the SENTINEL goal-aligned AI coding agent system.

## Available SDKs

- **TypeScript** - `@sentinel/sdk` - For Node.js and browser applications
- **Python** - `sentinel-sdk` - For Python applications and data pipelines

## Quick Start

### TypeScript

```bash
npm install @sentinel/sdk
```

```typescript
import { SentinelClient } from '@sentinel/sdk';

const client = new SentinelClient({ baseUrl: 'http://localhost:8080' });

const manifold = await client.createGoalManifold({
  description: 'Build a REST API',
  constraints: ['TypeScript', 'PostgreSQL']
});
```

### Python

```bash
pip install sentinel-sdk
```

```python
from sentinel_sdk import SentinelClient

async with SentinelClient("http://localhost:8080") as client:
    manifold = await client.create_goal_manifold(
        description="Build a REST API",
        constraints=["Python", "FastAPI"]
    )
```

## Features

- **Type-safe**: Full TypeScript definitions and Python type hints
- **Async-first**: Built on modern async/await patterns
- **Real-time**: WebSocket support for live updates
- **Comprehensive**: Covers all SENTINEL API endpoints

## Examples

See the `examples/` directory in each SDK for complete working examples.

## License

MIT License - see LICENSE for details
