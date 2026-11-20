import { test } from 'node:test';
import assert from 'node:assert/strict';
import { makeEnv } from './shim.js';
import { runAll } from '../src/tests.js';

test('tüm sistem testleri (Çekirdek, Program senkronu, Genel site, Yönetim paneli)', async () => {
  const env = makeEnv();
  const rep = await runAll(env, process.env.MODULE ? { module: process.env.MODULE } : {});
  const fails = rep.results.filter((r) => !r.ok);
  console.log(`${rep.passed} geçti, ${rep.failed} başarısız, ${rep.results.length} test, ${rep.ms} ms`);
  assert.equal(fails.length, 0, '\n' + fails.map((f) => `✗ [${f.module}] ${f.name}: ${f.error}`).join('\n'));
});
