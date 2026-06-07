/**
 * FreqPrompt v3 — Domain Adaptation (Sprint 4)
 *
 * Corpus upload → custom frequency table → hybrid scoring.
 * All data stored in IndexedDB for privacy.
 */

import { callWasm } from './frequency';

/* ═══════════════ Types ═══════════════ */

export interface DomainFreqTable {
  name: string;
  lang: string;
  total_tokens: number;
  unique_words: number;
  words: Record<string, { word: string; count: number; zipf: number }>;
}

export interface HybridResult {
  general_score: number;
  hybrid_score: number;
  token_count: number;
  domain_coverage: number;
}

/* ═══════════════ IndexedDB Storage ═══════════════ */

const DB_NAME = 'freqprompt';
const DOMAIN_STORE = 'domains';

async function getDb(): Promise<IDBDatabase> {
  return new Promise((resolve, reject) => {
    const req = indexedDB.open(DB_NAME, 2);
    req.onupgradeneeded = () => {
      const db = req.result;
      if (!db.objectStoreNames.contains(DOMAIN_STORE)) {
        db.createObjectStore(DOMAIN_STORE);
      }
      if (!db.objectStoreNames.contains('settings')) {
        db.createObjectStore('settings');
      }
    };
    req.onsuccess = () => resolve(req.result);
    req.onerror = () => reject(req.error);
  });
}

export async function saveDomainTable(table: DomainFreqTable): Promise<void> {
  const db = await getDb();
  return new Promise((resolve, reject) => {
    const tx = db.transaction(DOMAIN_STORE, 'readwrite');
    const store = tx.objectStore(DOMAIN_STORE);
    store.put(table, table.name);
    tx.oncomplete = () => resolve();
    tx.onerror = () => reject(tx.error);
  });
}

export async function loadDomainTable(name: string): Promise<DomainFreqTable | null> {
  const db = await getDb();
  return new Promise((resolve) => {
    const tx = db.transaction(DOMAIN_STORE, 'readonly');
    const store = tx.objectStore(DOMAIN_STORE);
    const req = store.get(name);
    req.onsuccess = () => resolve(req.result || null);
    req.onerror = () => resolve(null);
  });
}

export async function listDomainTables(): Promise<string[]> {
  const db = await getDb();
  return new Promise((resolve) => {
    const tx = db.transaction(DOMAIN_STORE, 'readonly');
    const store = tx.objectStore(DOMAIN_STORE);
    const req = store.getAllKeys();
    req.onsuccess = () => resolve(req.result as string[]);
    req.onerror = () => resolve([]);
  });
}

export async function deleteDomainTable(name: string): Promise<void> {
  const db = await getDb();
  return new Promise((resolve, reject) => {
    const tx = db.transaction(DOMAIN_STORE, 'readwrite');
    const store = tx.objectStore(DOMAIN_STORE);
    store.delete(name);
    tx.oncomplete = () => resolve();
    tx.onerror = () => reject(tx.error);
  });
}

/* ═══════════════ Corpus Processing ═══════════════ */

/**
 * Process raw text into tokens suitable for frequency table construction.
 */
export function tokenizeCorpus(text: string, lang: string): string[] {
  if (lang === 'zh') {
    // Chinese: split by punctuation and whitespace, keep multi-char segments
    return text
      .split(/[\s　。！？，、；：""''（）\[\]【】《》\n\r\t]+/)
      .filter(t => t.length > 0 && /[一-鿿A-Za-z0-9]/.test(t));
  } else {
    // English: whitespace + punctuation split
    return text
      .split(/[\s\.,;:!?'"()\[\]{}<>\n\r\t]+/)
      .filter(t => t.length > 0 && /[A-Za-z0-9]/.test(t))
      .map(t => t.toLowerCase());
  }
}

/**
 * Build a domain frequency table from raw corpus text.
 * Calls WASM to do the heavy lifting.
 */
export async function buildDomainTable(
  name: string,
  corpusText: string,
  lang: string
): Promise<DomainFreqTable> {
  const tokens = tokenizeCorpus(corpusText, lang);
  const table: DomainFreqTable = await callWasm('build_domain_freq_table', [{
    name,
    lang,
    tokens,
  }]);
  return table;
}

/**
 * Compute hybrid score (general + domain) for a sentence.
 */
export async function computeHybridScore(
  sentence: string,
  domainTableName: string,
  alpha: number = 0.6,
  beta: number = 0.4
): Promise<HybridResult | null> {
  const table = await loadDomainTable(domainTableName);
  if (!table) return null;

  return callWasm('hybrid_sentence_score', [{
    sentence,
    domain_table_json: JSON.stringify(table),
    alpha,
    beta,
  }]);
}

/* ═══════════════ Domain Manager UI ═══════════════ */

/**
 * Render domain management UI into a container element.
 */
export function renderDomainManager(
  container: HTMLElement,
  onDomainSelect: (name: string | null) => void
): void {
  container.innerHTML = `
    <div class="domain-manager">
      <div class="domain-header">
        <h3>领域语料库</h3>
        <button class="btn-secondary" id="btn-upload-corpus">上传语料</button>
      </div>
      <div id="domain-list" class="domain-list"></div>
      <input type="file" id="corpus-file-input" accept=".txt,.json,.jsonl,.csv" style="display:none">
    </div>
  `;

  const listEl = container.querySelector('#domain-list')!;
  const fileInput = container.querySelector('#corpus-file-input') as HTMLInputElement;
  const uploadBtn = container.querySelector('#btn-upload-corpus')!;

  async function refreshList() {
    const names = await listDomainTables();
    if (names.length === 0) {
      listEl.innerHTML = '<div class="domain-empty">暂无自定义语料库。上传文本文件构建领域频率表。</div>';
      return;
    }

    listEl.innerHTML = '';
    for (const name of names) {
      const table = await loadDomainTable(name);
      const row = document.createElement('div');
      row.className = 'domain-item';
      row.innerHTML = `
        <div class="domain-item-info">
          <span class="domain-item-name">${name}</span>
          <span class="domain-item-stats">${table?.unique_words ?? 0} 词 / ${table?.total_tokens ?? 0} 令牌</span>
        </div>
        <div class="domain-item-actions">
          <button class="btn-icon" data-action="select" title="使用此领域">✓</button>
          <button class="btn-icon" data-action="delete" title="删除">✕</button>
        </div>
      `;
      row.querySelector('[data-action="select"]')!.addEventListener('click', () => {
        onDomainSelect(name);
        listEl.querySelectorAll('.domain-item').forEach(el => el.classList.remove('selected'));
        row.classList.add('selected');
      });
      row.querySelector('[data-action="delete"]')!.addEventListener('click', async () => {
        if (confirm(`确定删除语料库"${name}"？`)) {
          await deleteDomainTable(name);
          onDomainSelect(null);
          refreshList();
        }
      });
      listEl.appendChild(row);
    }
  }

  uploadBtn.addEventListener('click', () => fileInput.click());
  fileInput.addEventListener('change', async () => {
    const file = fileInput.files?.[0];
    if (!file) return;

    const text = await file.text();
    const lang = /[一-鿿]/.test(text.slice(0, 100)) ? 'zh' : 'en';
    const name = file.name.replace(/\.[^.]+$/, '');

    uploadBtn.textContent = '构建中…';
    (uploadBtn as HTMLButtonElement).disabled = true;

    try {
      const table = await buildDomainTable(name, text, lang);
      await saveDomainTable(table);
      refreshList();
    } catch (e) {
      alert(`构建失败: ${e}`);
    } finally {
      uploadBtn.textContent = '上传语料';
      (uploadBtn as HTMLButtonElement).disabled = false;
      fileInput.value = '';
    }
  });

  refreshList();
}
