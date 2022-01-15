use std::collections::HashMap;

use crate::errors::Result;
use crate::internal::client_fabric::ClientFabric;
use crate::internal::discovery::StaticDiscovery;
use crate::internal::query::Query;
use crate::internal::transaction::Transaction;
use crate::types::{YdbList, YdbStruct, YdbValue};
use crate::internal::transaction::Mode;
use http::Uri;
use std::iter::FromIterator;
use std::str::FromStr;
use ydb_protobuf::generated::ydb::discovery::{ListEndpointsRequest, WhoAmIRequest};
use crate::internal::test_helpers::CONNECTION_INFO;
use crate::internal::transaction::Mode::SerializableReadWrite;

fn create_client() -> Result<ClientFabric> {
    let _endpoint_uri = Uri::from_str(CONNECTION_INFO.discovery_endpoint.as_str())?;

    let discovery = StaticDiscovery::from_str(CONNECTION_INFO.discovery_endpoint.as_str())?;

    return ClientFabric::new(
        CONNECTION_INFO.credentials.clone(),
        CONNECTION_INFO.database.clone(),
        Box::new(discovery),
    );
}

#[tokio::test]
async fn create_session() -> Result<()> {
    let res = create_client()?.table_client().create_session().await?;
    println!("session: {:?}", res);
    Ok(())
}

#[tokio::test]
async fn endpoints() -> Result<()> {
    let _res = create_client()?
        .endpoints(ListEndpointsRequest::default())
        .await?;
    println!("{:?}", _res);
    Ok(())
}

#[tokio::test]
async fn execute_data_query() -> Result<()> {
    let client = create_client()?;
    let mut transaction = client
        .table_client()
        .create_autocommit_transaction(Mode::OnlineReadonly);
    let res = transaction.query("SELECT 1+1".into()).await?;
    println!("result: {:?}", &res);
    assert_eq!(
        YdbValue::Int32(2),
        res.first()
            .unwrap()
            .rows()
            .next()
            .unwrap()
            .remove_field(0)
            .unwrap()
    );
    Ok(())
}

#[tokio::test]
async fn execute_data_query_field_name() -> Result<()> {
    let client = create_client()?;
    let mut transaction = client
        .table_client()
        .create_autocommit_transaction(Mode::OnlineReadonly);
    let res = transaction.query("SELECT 1+1 as s".into()).await?;
    println!("result: {:?}", &res);
    assert_eq!(
        YdbValue::Int32(2),
        res.first()
            .unwrap()
            .rows()
            .next()
            .unwrap()
            .remove_field_by_name("s")
            .unwrap()
    );
    Ok(())
}

#[tokio::test]
async fn execute_data_query_params() -> Result<()> {
    let client = create_client()?;
    let mut transaction = client
        .table_client()
        .create_autocommit_transaction(Mode::OnlineReadonly);
    let mut params = HashMap::new();
    params.insert("$v".to_string(), YdbValue::Int32(3));
    let res = transaction
        .query(
            Query::new()
                .with_query(
                    "
                DECLARE $v AS Int32;
                SELECT $v+$v
        "
                        .into(),
                )
                .with_params(params),
        )
        .await?;
    println!("result: {:?}", res);
    assert_eq!(
        YdbValue::Int32(6),
        res.first()
            .unwrap()
            .rows()
            .next()
            .unwrap()
            .remove_field(0)
            .unwrap()
    );
    Ok(())
}

#[tokio::test]
async fn interactive_transaction()->Result<()>{
    let client = create_client()?;
    let mut tx_auto = client.table_client().create_autocommit_transaction(SerializableReadWrite);

    let mut tx = client.table_client().create_interactive_transaction();
    tx.query(Query::new().with_query("DELETE FROM test_values".into())).await?;
    tx.commit().await?;

    let mut tx = client.table_client().create_interactive_transaction();
    tx.query(Query::new().with_query("UPSERT INTO test_values (id, vInt64) VALUES (1, 2)".into())).await?;
    tx.query(Query::new()
        .with_query("
                DECLARE $key AS Int64;
                DECLARE $val AS Int64;

                UPSERT INTO test_values (id, vInt64) VALUES ($key, $val)
            ".into())
        .with_params(HashMap::from([
            ("$key".into(), YdbValue::Int64(2)),
            ("$val".into(), YdbValue::Int64(3)),
        ]))
    ).await?;

    // check table before commit
    let auto_res = tx_auto.query(Query::new().with_query("SELECT vInt64 FROM test_values WHERE id=1".into())).await?;
    assert!(auto_res.first().unwrap().rows().next().is_none());

    tx.commit().await?;

    // check table after commit
    let auto_res = tx_auto.query(Query::new().with_query("SELECT vInt64 FROM test_values WHERE id=1".into())).await?;
    assert_eq!(YdbValue::optional_from(YdbValue::Int64(0), Some(YdbValue::Int64(2)))?,
               auto_res
                   .first()
                   .unwrap()
                   .rows()
                   .next()
                   .unwrap()
                   .remove_field_by_name("vInt64")
                   .unwrap()
    );

    return Ok(());
}

#[tokio::test]
async fn select_int() -> Result<()> {
    let client = create_client()?;
    let v = YdbValue::Int32(123);

    let mut transaction = client
        .table_client()
        .create_autocommit_transaction(Mode::OnlineReadonly);
    let res = transaction
        .query(
            Query::new()
                .with_query(
                    "
DECLARE $test AS Int32;

SELECT $test AS test;
"
                        .into(),
                )
                .with_params(HashMap::from_iter([("$test".into(), v.clone())])),
        )
        .await?;

    let res = res.results.into_iter().next().unwrap();
    assert_eq!(1, res.columns().len());
    assert_eq!(v, res.rows().next().unwrap().remove_field_by_name("test")?);

    return Ok(());
}

#[tokio::test]
async fn select_optional() -> Result<()> {
    let client = create_client()?;
    let mut transaction = client
        .table_client()
        .create_autocommit_transaction(Mode::OnlineReadonly);
    let res = transaction
        .query(
            Query::new()
                .with_query(
                    "
DECLARE $test AS Optional<Int32>;

SELECT $test AS test;
"
                        .into(),
                )
                .with_params(HashMap::from_iter([(
                    "$test".into(),
                    YdbValue::optional_from(YdbValue::Int32(0), Some(YdbValue::Int32(3)))?,
                )])),
        )
        .await?;

    let res = res.results.into_iter().next().unwrap();
    assert_eq!(1, res.columns().len());
    assert_eq!(
        YdbValue::optional_from(YdbValue::Int32(0), Some(YdbValue::Int32(3)))?,
        res.rows().next().unwrap().remove_field_by_name("test")?
    );

    return Ok(());
}

#[tokio::test]
async fn select_list() -> Result<()> {
    let client = create_client()?;
    let mut transaction = client
        .table_client()
        .create_autocommit_transaction(Mode::OnlineReadonly);
    let res = transaction
        .query(
            Query::new()
                .with_query(
                    "
DECLARE $l AS List<Int32>;

SELECT $l AS l;
"
                        .into(),
                )
                .with_params(HashMap::from_iter([(
                    "$l".into(),
                    YdbValue::List(Box::new(YdbList {
                        t: YdbValue::Int32(0),
                        values: Vec::from([
                            YdbValue::Int32(1),
                            YdbValue::Int32(2),
                            YdbValue::Int32(3),
                        ]),
                    })),
                )])),
        )
        .await?;
    println!("{:?}", res);
    let res = res.results.into_iter().next().unwrap();
    assert_eq!(1, res.columns().len());
    assert_eq!(
        YdbValue::list_from(
            YdbValue::Int32(0),
            vec![YdbValue::Int32(1), YdbValue::Int32(2), YdbValue::Int32(3)]
        )?,
        res.rows().next().unwrap().remove_field_by_name("l")?
    );
    Ok(())
}

#[tokio::test]
async fn select_struct() -> Result<()> {
    let client = create_client()?;
    let mut transaction = client
        .table_client()
        .create_autocommit_transaction(Mode::OnlineReadonly);
    let res = transaction
        .query(
            Query::new()
                .with_query(
                    "
DECLARE $l AS List<Struct<
    a: Int64
>>;

SELECT
    SUM(a) AS s
FROM
    AS_TABLE($l);
;
"
                        .into(),
                )
                .with_params(HashMap::from_iter([(
                    "$l".into(),
                    YdbValue::List(Box::new(YdbList {
                        t: YdbValue::Struct(YdbStruct::from_names_and_values(
                            vec!["a".into()],
                            vec![YdbValue::Int64(0)],
                        )?),
                        values: vec![
                            YdbValue::Struct(YdbStruct::from_names_and_values(
                                vec!["a".into()],
                                vec![YdbValue::Int64(1)],
                            )?),
                            YdbValue::Struct(YdbStruct::from_names_and_values(
                                vec!["a".into()],
                                vec![YdbValue::Int64(2)],
                            )?),
                            YdbValue::Struct(YdbStruct::from_names_and_values(
                                vec!["a".into()],
                                vec![YdbValue::Int64(3)],
                            )?),
                        ],
                    })),
                )])),
        )
        .await?;
    println!("{:?}", res);
    let res = res.results.into_iter().next().unwrap();
    assert_eq!(1, res.columns().len());

    assert_eq!(
        YdbValue::optional_from(YdbValue::Int64(0), Some(YdbValue::Int64(6)))?,
        res.rows().next().unwrap().remove_field_by_name("s")?
    );
    Ok(())
}

#[tokio::test]
async fn select_int64_null4() -> Result<()> {
    let client = create_client()?;
    let mut transaction = client
        .table_client()
        .create_autocommit_transaction(Mode::OnlineReadonly);
    let res = transaction
        .query(
            Query::new().with_query(
                "
SELECT CAST(NULL AS Optional<Int64>)
;
"
                    .into(),
            ),
        )
        .await?;
    println!("{:?}", res);
    let res = res.results.into_iter().next().unwrap();
    assert_eq!(1, res.columns().len());

    assert_eq!(
        YdbValue::optional_from(YdbValue::Int64(0), None)?,
        res.rows().next().unwrap().remove_field(0)?
    );
    Ok(())
}

#[tokio::test]
async fn select_void_null() -> Result<()> {
    let client = create_client()?;
    let mut transaction = client
        .table_client()
        .create_autocommit_transaction(Mode::OnlineReadonly);
    let res = transaction
        .query(
            Query::new().with_query(
                "
SELECT NULL
;
"
                    .into(),
            ),
        )
        .await?;
    println!("{:?}", res);
    let res = res.results.into_iter().next().unwrap();
    assert_eq!(1, res.columns().len());

    assert_eq!(
        YdbValue::optional_from(YdbValue::Void, None)?,
        res.rows().next().unwrap().remove_field(0)?
    );
    Ok(())
}

#[tokio::test]
async fn who_am_i() -> Result<()> {
    let res = create_client()?.who_am_i(WhoAmIRequest::default()).await?;
    assert!(res.user.len() > 0);
    Ok(())
}
