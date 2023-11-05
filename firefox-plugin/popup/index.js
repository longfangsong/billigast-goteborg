addEventListener("load", function () {
    this.setInterval(() => {
        browser.storage.local.get('records').then(({ records }) => {
            let sql = '';
            for (let i = 0; i < records.length; i++) {
                const record = records[i];
                sql += `INSERT INTO ButikCommodity(butik_id, commodity_id, fetch_id)
                        SELECT ${record.butik}, ${record.commodity}, '${record.fetch_id}'
                        WHERE NOT EXISTS (SELECT 1 FROM ButikCommodity WHERE
                            butik_id = ${record.butik} AND
                            commodity_id = ${record.commodity} AND
                            fetch_id = '${record.fetch_id}'
                        );\n`;
            }
            document.getElementById('sql').innerText = sql;
        });
    }, 1000);
    this.document.getElementById('copy').onclick = () => {
        let sqlElement = document.getElementById('sql');
        navigator.clipboard.writeText(sqlElement.innerText).then(
            () => {
                browser.storage.local.set({ records: [] });
                sqlElement.innerText = '';
            },
        );
    }
});