/**
 * SENTINEL TypeScript SDK - Basic Usage Example
 * 
 * This example demonstrates how to use the SENTINEL SDK to:
 * - Create a goal manifold
 * - Add goals with success criteria
 * - Track progress
 * - Validate alignment
 */

import { SentinelClient, Intent, Goal, GoalStatus } from '@sentinel/sdk';

// Configuration
const SENTINEL_URL = process.env.SENTINEL_URL || 'http://localhost:8080';

async function main() {
  // Initialize client
  const client = new SentinelClient({
    baseUrl: SENTINEL_URL,
    timeout: 30000,
  });

  console.log('🔗 Connected to SENTINEL at', SENTINEL_URL);

  // Step 1: Create a new goal manifold for a web application project
  const intent: Intent = {
    description: 'Build a modern e-commerce platform with user authentication, product catalog, and payment integration',
    constraints: [
      'Use Next.js 14 with App Router',
      'PostgreSQL database with Prisma ORM',
      'Stripe for payment processing',
      'Tailwind CSS for styling',
      'Test coverage > 80%',
    ],
    languages: ['TypeScript', 'SQL'],
    frameworks: ['Next.js', 'Prisma', 'Tailwind CSS'],
    targetPlatform: 'web',
  };

  const manifold = await client.createGoalManifold(intent);
  console.log(`✅ Created goal manifold: ${manifold.id}`);
  console.log(`   Root intent: ${manifold.rootIntent.description}`);

  // Step 2: Add project setup goal
  const setupGoal = await client.addGoal(manifold.id, {
    description: 'Initialize project structure and dependencies',
    successCriteria: [
      { type: 'file_exists', params: { path: 'package.json' } },
      { type: 'file_exists', params: { path: 'tsconfig.json' } },
      { type: 'file_exists', params: { path: 'prisma/schema.prisma' } },
    ],
    dependencies: [],
    antiDependencies: [],
    complexityEstimate: {
      mean: 3.0,
      stdDev: 0.5,
      min: 2.0,
      max: 4.0,
      distributionType: 'normal',
    },
    valueToRoot: 0.1,
    status: 'ready' as GoalStatus,
    validationTests: [],
    metadata: {
      tags: ['setup', 'configuration'],
      notes: 'Foundation for the entire project',
      priority: 1.0,
    },
  });
  console.log(`✅ Added setup goal: ${setupGoal.id}`);

  // Step 3: Add authentication goal (depends on setup)
  const authGoal = await client.addGoal(manifold.id, {
    description: 'Implement JWT-based authentication system',
    successCriteria: [
      { type: 'tests_passing', params: { suite: 'auth', minCoverage: 0.85 } },
      { type: 'file_exists', params: { path: 'src/lib/auth.ts' } },
      { type: 'api_available', params: { endpoint: '/api/auth/login', method: 'POST' } },
    ],
    dependencies: [setupGoal.id],
    antiDependencies: [],
    complexityEstimate: {
      mean: 7.0,
      stdDev: 1.0,
      min: 5.0,
      max: 9.0,
      distributionType: 'normal',
    },
    valueToRoot: 0.25,
    status: 'pending' as GoalStatus,
    validationTests: ['test/auth/login.test.ts', 'test/auth/register.test.ts'],
    metadata: {
      tags: ['authentication', 'security', 'jwt'],
      notes: 'Critical security component',
      priority: 0.9,
    },
  });
  console.log(`✅ Added authentication goal: ${authGoal.id}`);

  // Step 4: Add product catalog goal
  const catalogGoal = await client.addGoal(manifold.id, {
    description: 'Build product catalog with search and filtering',
    successCriteria: [
      { type: 'tests_passing', params: { suite: 'catalog', minCoverage: 0.8 } },
      { type: 'api_available', params: { endpoint: '/api/products', method: 'GET' } },
    ],
    dependencies: [setupGoal.id],
    antiDependencies: [],
    complexityEstimate: {
      mean: 6.0,
      stdDev: 1.5,
      min: 4.0,
      max: 8.0,
      distributionType: 'normal',
    },
    valueToRoot: 0.2,
    status: 'pending' as GoalStatus,
    validationTests: ['test/catalog/products.test.ts'],
    metadata: {
      tags: ['products', 'search', 'catalog'],
      notes: 'Core e-commerce functionality',
      priority: 0.8,
    },
  });
  console.log(`✅ Added catalog goal: ${catalogGoal.id}`);

  // Step 5: Add payment integration goal (depends on auth)
  const paymentGoal = await client.addGoal(manifold.id, {
    description: 'Integrate Stripe payment processing',
    successCriteria: [
      { type: 'tests_passing', params: { suite: 'payment', minCoverage: 0.9 } },
      { type: 'api_available', params: { endpoint: '/api/payment/checkout', method: 'POST' } },
    ],
    dependencies: [authGoal.id, catalogGoal.id],
    antiDependencies: [],
    complexityEstimate: {
      mean: 8.0,
      stdDev: 1.0,
      min: 6.0,
      max: 10.0,
      distributionType: 'normal',
    },
    valueToRoot: 0.3,
    status: 'pending' as GoalStatus,
    validationTests: ['test/payment/stripe.test.ts'],
    metadata: {
      tags: ['payment', 'stripe', 'checkout'],
      notes: 'Revenue-critical feature',
      priority: 1.0,
    },
  });
  console.log(`✅ Added payment goal: ${paymentGoal.id}`);

  // Step 6: Mark setup goal as completed
  await client.updateGoalStatus(manifold.id, setupGoal.id, 'completed');
  console.log(`✅ Marked setup goal as completed`);

  // Step 7: Check overall progress
  const updatedManifold = await client.getGoalManifold(manifold.id);
  console.log(`\n📊 Project Progress:`);
  console.log(`   Completion: ${(updatedManifold.completionPercentage * 100).toFixed(1)}%`);
  console.log(`   Total goals: ${updatedManifold.goals.length}`);

  // Step 8: Validate alignment
  const alignment = await client.getAlignmentScore(manifold.id);
  console.log(`\n🎯 Alignment Score: ${alignment.score.toFixed(1)}/100`);
  console.log(`   Confidence: ${(alignment.confidence * 100).toFixed(1)}%`);
  
  if (alignment.violations.length > 0) {
    console.log(`   ⚠️  Violations found: ${alignment.violations.length}`);
    for (const v of alignment.violations) {
      console.log(`      - ${v.description} (${v.severity})`);
    }
  } else {
    console.log(`   ✅ No alignment violations`);
  }

  // Step 9: Subscribe to real-time updates
  console.log('\n🔄 Subscribing to real-time updates...');
  const unsubscribe = client.subscribeToUpdates(manifold.id, (update) => {
    console.log(`📡 Update received: ${update.type} at ${update.timestamp}`);
  });

  // Keep the subscription alive for a bit
  await new Promise((resolve) => setTimeout(resolve, 5000));
  
  // Cleanup
  unsubscribe();
  console.log('\n👋 Done!');
}

// Run the example
main().catch((error) => {
  console.error('❌ Error:', error);
  process.exit(1);
});
