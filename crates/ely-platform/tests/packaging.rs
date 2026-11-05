use ely_platform::AppId::{self, *};

#[test]
fn packaging_table_lists_the_apps_in_prd_order() {
    let apps: [AppId; 8] = [Doc, Table, Show, Pdf, Mail, Tree, Mark, Agent];
    let table = include_str!("../../../packaging/apps.tsv");
    let rows: Vec<Vec<&str>> = table
        .lines()
        .map(|line| line.split('\t').collect())
        .collect();

    assert_eq!(rows.len(), apps.len());
    for (row, app) in rows.iter().zip(apps) {
        assert_eq!(row[..2], [app.slug(), app.name()]);
        assert!(
            row[2].ends_with(';'),
            "{} has no desktop categories",
            app.slug()
        );
        // Agent ships in the second batch.
        let batch = if matches!(app, Agent) { "2" } else { "1" };
        assert_eq!(row[3], batch, "{} is in the wrong batch", app.slug());
        // Each engine it bundles keeps its licence.
        let license = match app {
            Doc | Table | Show => "MIT AND MPL-2.0",
            Pdf => "MIT AND AGPL-3.0-or-later",
            Tree => "MIT AND GPL-2.0-only",
            Mark => "MIT AND AGPL-3.0-or-later AND GPL-2.0-only",
            Mail | Agent => "MIT",
        };
        assert_eq!(row[4], license, "{}", app.slug());
        assert!(!row[5].is_empty(), "{} has no summary", app.slug());
    }
}
