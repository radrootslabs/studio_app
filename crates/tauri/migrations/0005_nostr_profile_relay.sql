CREATE TABLE IF NOT EXISTS nostr_profile_relay (
    tb_pr_rl_0 CHAR(36),
    tb_pr_rl_1 CHAR(36),
    FOREIGN KEY (tb_pr_rl_0) REFERENCES nostr_profile(id) ON DELETE CASCADE,
    FOREIGN KEY (tb_pr_rl_1) REFERENCES nostr_relay(id) ON DELETE CASCADE,
    PRIMARY KEY (tb_pr_rl_0, tb_pr_rl_1)
);