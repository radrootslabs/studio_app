alter table buyer_carts
    add column buyer_order_note_public_confirmed integer not null default 0;

alter table orders
    add column buyer_order_note_public_confirmed integer not null default 0;
