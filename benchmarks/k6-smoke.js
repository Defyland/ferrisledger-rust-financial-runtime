import http from 'k6/http';
import { check, sleep } from 'k6';

export const options = {
  vus: 1,
  iterations: 3,
  summaryTrendStats: ['avg', 'min', 'med', 'p(50)', 'p(90)', 'p(95)', 'p(99)', 'max'],
  thresholds: {
    http_req_failed: ['rate<0.01'],
    http_req_duration: ['p(95)<500'],
  },
};

const baseUrl = __ENV.BASE_URL || 'http://localhost:8080';
const apiKey = __ENV.API_KEY || 'dev-secret-local';

export default function () {
  const accountId = `smoke_${Date.now()}_${__VU}_${__ITER}`;
  const openCorrelationId = `corr_open_${accountId}`;
  const depositCorrelationId = `corr_deposit_${accountId}`;
  const headers = {
    'content-type': 'application/json',
    'x-api-key': apiKey,
    'x-request-id': `req_${accountId}`,
  };
  const open = http.post(`${baseUrl}/v1/accounts`, JSON.stringify({
    tenant_id: 'tenant_bench',
    account_id: accountId,
    currency: 'BRL',
    account_holder_name: 'Benchmark User',
    correlation_id: openCorrelationId,
  }), { headers });
  check(open, {
    'open account succeeded': (r) => r.status === 200,
    'open response carries request id': (r) => r.headers['X-Request-Id'] === `req_${accountId}`,
    'open response carries correlation id': (r) => r.headers['X-Correlation-Id'] === openCorrelationId,
  });

  const deposit = http.post(`${baseUrl}/v1/accounts/${accountId}/deposits`, JSON.stringify({
    tenant_id: 'tenant_bench',
    amount_cents: 1000,
    currency: 'BRL',
    idempotency_key: `deposit_${accountId}`,
    correlation_id: depositCorrelationId,
  }), { headers });
  check(deposit, {
    'deposit succeeded': (r) => r.status === 200,
    'deposit response carries request id': (r) => r.headers['X-Request-Id'] === `req_${accountId}`,
    'deposit response carries correlation id': (r) => r.headers['X-Correlation-Id'] === depositCorrelationId,
  });
  sleep(1);
}
