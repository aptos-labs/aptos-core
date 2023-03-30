-- Your SQL goes here
ALTER TABLE collection_datas
ADD COLUMN table_handle VARCHAR(66) NOT NULL;
ALTER TABLE current_collection_datas
ADD COLUMN table_handle VARCHAR(66) NOT NULL;
CREATE INDEX curr_cd_th_index ON current_collection_datas (table_handle);