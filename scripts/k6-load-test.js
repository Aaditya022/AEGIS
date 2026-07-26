// AEGIS Load Test Script for k6
// Run: k6 run -e URL=http://localhost:8000 --vus 50 --duration 5m scripts/k6-load-test.js

import http from 'k6/http';
import { check, sleep } from 'k6';
import { Rate } from 'k6/metrics';

const BASE_URL = __ENV.URL || 'http://localhost:8000';

export const errorRate = new Rate('errors');

export const options = {
  stages: [
    { duration: '1m', target: 10 },    // Ramp up
    { duration: '3m', target: 50 },    // Sustained load
    { duration: '1m', target: 0 },     // Ramp down
  ],
  thresholds: {
    http_req_duration: ['p(95)<5000'], // 95% of requests < 5s
    errors: ['rate<0.05'],             // Error rate < 5%
  },
};

export default function () {
  const payload = JSON.stringify({
    model: 'gpt-4',
    prompt: 'Summarize the quarterly report',
  });

  const params = {
    headers: {
      'Content-Type': 'application/json',
      'X-AEGIS-Agent-ID': `load-test-agent-${__VU}`,
    },
  };

  const res = http.post(`${BASE_URL}/v1/route`, payload, params);

  check(res, {
    'status is 200': (r) => r.status === 200,
    'response time < 500ms': (r) => r.timings.duration < 500,
  });

  errorRate.add(res.status !== 200);
  sleep(0.1);
}
