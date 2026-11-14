-- Schema inicial (contrato congelado). Datas: ISO-8601 UTC; due_date 'YYYY-MM-DD' (America/Sao_Paulo).

CREATE TABLE account (
  id INTEGER PRIMARY KEY,
  library_base_url TEXT NOT NULL,
  library_name TEXT NOT NULL,
  login TEXT NOT NULL,
  password TEXT, -- NULL = modo "sem salvar senha"
  mode TEXT NOT NULL CHECK (mode IN ('auto','manual')),
  status TEXT NOT NULL DEFAULT 'active' CHECK (status IN ('active','paused','invalid_password','blocked')),
  renew_days_before INTEGER NOT NULL DEFAULT 1 CHECK (renew_days_before BETWEEN 0 AND 3),
  remind_days_before INTEGER NOT NULL DEFAULT 2 CHECK (remind_days_before BETWEEN 0 AND 7),
  last_sync_at TEXT,
  last_renewal_at TEXT,
  next_renewal_at TEXT,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  UNIQUE (library_base_url, login)
);

CREATE TABLE book (
  id INTEGER PRIMARY KEY,
  account_id INTEGER NOT NULL REFERENCES account(id) ON DELETE CASCADE,
  item_key TEXT NOT NULL,
  title TEXT NOT NULL,
  author TEXT,
  call_number TEXT,
  library_branch TEXT,
  first_seen_at TEXT NOT NULL,
  UNIQUE (account_id, item_key)
);

CREATE TABLE loan (
  id INTEGER PRIMARY KEY,
  book_id INTEGER NOT NULL REFERENCES book(id) ON DELETE CASCADE,
  loaned_at TEXT,
  due_date TEXT NOT NULL,
  renewals_count INTEGER NOT NULL DEFAULT 0,
  renewals_limit INTEGER,
  status TEXT NOT NULL CHECK (status IN ('active','returned','overdue')),
  auto_renew INTEGER NOT NULL DEFAULT 1,
  returned_at TEXT,
  updated_at TEXT NOT NULL
);

CREATE TABLE renewal_event (
  id INTEGER PRIMARY KEY,
  loan_id INTEGER NOT NULL REFERENCES loan(id) ON DELETE CASCADE,
  attempted_at TEXT NOT NULL,
  trigger TEXT NOT NULL CHECK (trigger IN ('auto','manual','catch_up')),
  result TEXT NOT NULL CHECK (result IN ('success','reserved','limit_reached','blocked','overdue','network_error','parse_error','captcha','rate_limited')),
  old_due_date TEXT,
  new_due_date TEXT,
  message TEXT
);

CREATE TABLE schedule (
  id INTEGER PRIMARY KEY,
  account_id INTEGER NOT NULL REFERENCES account(id) ON DELETE CASCADE,
  loan_id INTEGER REFERENCES loan(id) ON DELETE CASCADE,
  kind TEXT NOT NULL CHECK (kind IN ('renew','sync','remind')),
  run_at TEXT NOT NULL,
  status TEXT NOT NULL CHECK (status IN ('pending','paused','done','cancelled','failed')),
  origin TEXT NOT NULL CHECK (origin IN ('system','user')),
  updated_at TEXT NOT NULL
);

CREATE TABLE request_log (
  id INTEGER PRIMARY KEY,
  account_id INTEGER,
  host TEXT NOT NULL,
  at TEXT NOT NULL,
  kind TEXT NOT NULL,
  session_id TEXT NOT NULL,
  http_status INTEGER,
  outcome TEXT NOT NULL
);

CREATE TABLE breaker_state (
  scope TEXT PRIMARY KEY,
  consecutive_failures INTEGER NOT NULL DEFAULT 0,
  open_until TEXT,
  reason TEXT
);

CREATE TABLE setting (
  key TEXT PRIMARY KEY,
  value TEXT NOT NULL
);

CREATE INDEX idx_book_account ON book(account_id);
CREATE INDEX idx_loan_book ON loan(book_id);
CREATE INDEX idx_loan_status ON loan(status);
CREATE INDEX idx_loan_due ON loan(due_date);
CREATE INDEX idx_event_loan ON renewal_event(loan_id);
CREATE INDEX idx_event_attempted ON renewal_event(attempted_at);
CREATE INDEX idx_schedule_account ON schedule(account_id);
CREATE INDEX idx_schedule_loan ON schedule(loan_id);
CREATE INDEX idx_schedule_status_run ON schedule(status, run_at);
CREATE INDEX idx_request_log_at ON request_log(at);
CREATE INDEX idx_request_log_account ON request_log(account_id, at);
CREATE INDEX idx_request_log_host ON request_log(host, at);
