// AEGIS Gateway Load Test
// Run: k6 run --vus 50 --duration 5m tests/benchmarks/k6-gateway.js
//
// Metrics captured:
//   - http_req_duration (P50, P95, P99)
//   - http_reqs (throughput)
//   - iterations per second
//   - error rate

import http from 'k6/http';
import { check, sleep } from 'k6';
import { Rate, Trend, Counter } from 'k6/metrics';

const BASE_URL = __ENV.AEGIS_GATEWAY_URL || 'http://localhost:8000';

export const errorRate = new Rate('errors');
export const latencyTrend = new Trend('latency');
export const requestCount = new Counter('aegis_requests');

export const options = {
  stages: [
    { duration: '30s', target: 10 },    // Ramp up
    { duration: '1m', target: 50 },      // Sustained
    { duration: '1m', target: 100 },     // Peak
    { duration: '30s', target: 0 },      // Ramp down
  ],
  thresholds: {
    http_req_duration: ['p(95)<5000'],   // 95% under 5s
    'http_req_duration{type:route}': ['p(95)<100'],  // route queries under 100ms
    errors: ['rate<0.05'],               // Error rate < 5%
  },
};

export default function () {
  const vu_id = __VU;
  const agent_id = `bench-agent-${vu_id}`;

  // 1. Route lookup
  const routePayload = JSON.stringify({
    model: 'gpt-4',
    messages: [{ role: 'user', content: 'test' }],
  });

  let res = http.post(`${BASE_URL}/v1/route`, routePayload, {
    headers: {
      'Content-Type': 'application/json',
      'X-AEGIS-Agent-ID': agent_id,
    },
    tags: { type: 'route' },
  });

  check(res, {
    'route status 200': (r) => r.status === 200,
  });
  latencyTrend.add(res.timings.duration);
  errorRate.add(res.status !== 200);
  requestCount.add(1);

  // 2. Chat completion proxy
  const chatPayload = JSON.stringify({
    model: 'gpt-4',
    messages: [
      { role: 'system', content: 'You are a helpful assistant.' },
      { role: 'user', content: 'What is the capital of France?' },
    ],
    max_tokens: 10,
    temperature: 0,
  });

  res = http.post(`${BASE_URL}/v1/chat/completions`, chatPayload, {
    headers: {
      'Content-Type': 'application/json',
      'X-AEGIS-Agent-ID': agent_id,
    },
    tags: { type: 'chat' },
  });

  check(res, {
    'chat status 200': (r) => r.status === 200,
  });
  latencyTrend.add(res.timings.duration);
  errorRate.add(res.status !== 200);
  requestCount.add(1);

  sleep(0.1);
}

export function handleSummary(data) {
  const metrics = {
    throughput: data.metrics.http_reqs.values.rate,
    latency_p50: data.metrics.http_req_duration.values.p(50),
    latency_p95: data.metrics.http_req_duration.values.p(95),
    latency_p99: data.metrics.http_req_duration.values.p(99),
    error_rate: data.metrics.errors.values.rate,
    total_requests: data.metrics.http_reqs.values.count,
  };

  console.log('\n=== AEGIS Gateway Benchmark Results ===');
  console.log(`Throughput:       ${metrics.throughput.toFixed(0)} req/s`);
  console.log(`P50 Latency:      ${metrics.latency_p50.toFixed(2)} ms`);
  console.log(`P95 Latency:      ${metrics.latency_p95.toFixed(2)} ms`);
  console.log(`P99 Latency:      ${metrics.latency_p99.toFixed(2)} ms`);
  console.log(`Error Rate:       ${(metrics.error_rate * 100).toFixed(2)}%`);
  console.log(`Total Requests:   ${metrics.total_requests}`);
  console.log('=======================================\n');

  return {
    'stdout': JSON.stringify(metrics),
  };
}
