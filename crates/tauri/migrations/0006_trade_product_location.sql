CREATE TABLE IF NOT EXISTS trade_product_location (
    tb_tp_lg_0 CHAR(36),
    tb_tp_lg_1 CHAR(36),
    FOREIGN KEY (tb_tp_lg_0) REFERENCES trade_product(id) ON DELETE CASCADE,
    FOREIGN KEY (tb_tp_lg_1) REFERENCES location_gcs(id) ON DELETE CASCADE,
    PRIMARY KEY (tb_tp_lg_0, tb_tp_lg_1)
);