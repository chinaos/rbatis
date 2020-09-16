
#[cfg(test)]
mod test {
    use fast_log::log::RuntimeType;
    use serde::Deserialize;
    use serde::Serialize;

    use rbatis::crud::{CRUD, CRUDEnable};
    use rbatis::plugin::page::{Page, PageRequest};
    use rbatis::rbatis::Rbatis;
    use rbatis_core::Error;
    use rbatis_core::types::chrono::NaiveDateTime;
    use rbatis_core::value::DateTimeNow;

    #[derive(CRUDEnable, Serialize, Deserialize, Clone, Debug)]
    pub struct BizActivity {
        pub id: Option<String>,
        pub name: Option<String>,
        pub pc_link: Option<String>,
        pub h5_link: Option<String>,
        pub pc_banner_img: Option<String>,
        pub h5_banner_img: Option<String>,
        pub sort: Option<String>,
        pub status: Option<i32>,
        pub remark: Option<String>,
        pub create_time: Option<NaiveDateTime>,
        pub version: Option<i32>,
        pub delete_flag: Option<i32>,
    }

// (可选) 手动实现，不使用上面的derive(CRUDEnable),可重写table_name方法。手动实现能支持IDE智能提示
// impl CRUDEnable for BizActivity {
//     type IdType = String;
// }

    pub async fn init_rbatis() -> Rbatis {
        fast_log::log::init_log("requests.log", &RuntimeType::Std);
        let rb = Rbatis::new();
        rb.link("sqlite://rbatis.sqlite").await.unwrap();

        // custom pool(自定义连接池)
        // let mut opt = PoolOptions::new();
        // opt.max_size = 20;
        // rb.link_opt("sqlite://rbatis.sqlite", &opt).await.unwrap();
        return rb;
    }

    #[test]
    pub fn test_save() {
        async_std::task::block_on(async {
            let rb = init_rbatis().await;
            let activity = BizActivity {
                id: Some("12312".to_string()),
                name: Some("123".to_string()),
                pc_link: None,
                h5_link: None,
                pc_banner_img: None,
                h5_banner_img: None,
                sort: Some("1".to_string()),
                status: Some(1),
                remark: None,
                create_time: Some(NaiveDateTime::now()),
                version: Some(1),
                delete_flag: Some(1),
            };
            let r = rb.save("", &activity).await;
            if r.is_err() {
                println!("{}", r.err().unwrap().to_string());
            }
        });
    }

    #[test]
    pub fn test_save_batch() {
        async_std::task::block_on(async {
            let rb = init_rbatis().await;
            let activity = BizActivity {
                id: Some("12312".to_string()),
                name: Some("test_1".to_string()),
                pc_link: None,
                h5_link: None,
                pc_banner_img: None,
                h5_banner_img: None,
                sort: None,
                status: Some(1),
                remark: None,
                create_time: Some(NaiveDateTime::now()),
                version: Some(1),
                delete_flag: Some(1),
            };
            let args = vec![activity.clone(), activity];
            let r = rb.save_batch("", &args).await;
            if r.is_err() {
                println!("{}", r.err().unwrap().to_string());
            }
        });
    }


    #[test]
    pub fn test_remove_batch_by_id() {
        async_std::task::block_on(async {
            let rb = init_rbatis().await;
            let r = rb.remove_batch_by_id::<BizActivity>("", &["1".to_string(), "2".to_string()]).await;
            if r.is_err() {
                println!("{}", r.err().unwrap().to_string());
            }
        });
    }


    #[test]
    pub fn test_remove_by_id() {
        async_std::task::block_on(async {
            let rb = init_rbatis().await;
            let r = rb.remove_by_id::<BizActivity>("", &"12312".to_string()).await;
            if r.is_err() {
                println!("{}", r.err().unwrap().to_string());
            }
        });
    }

    #[test]
    pub fn test_fetch_by_id() {
        async_std::task::block_on(async {
            let rb = init_rbatis().await;
            let r = rb.fetch_by_id::<Option<BizActivity>>("", &"12312".to_string()).await.unwrap();
            println!("{}", serde_json::to_string(&r).unwrap());
        });
    }

    #[test]
    pub fn test_update_by_wrapper() {
        async_std::task::block_on(async {
            let rb = init_rbatis().await;
            let activity = BizActivity {
                id: Some("12312".to_string()),
                name: None,
                pc_link: None,
                h5_link: None,
                pc_banner_img: None,
                h5_banner_img: None,
                sort: None,
                status: Some(1),
                remark: None,
                create_time: Some(NaiveDateTime::now()),
                version: Some(1),
                delete_flag: Some(1),
            };

            let w = rb.new_wrapper().eq("id", "12312").check().unwrap();
            let r = rb.update_by_wrapper("", &activity, &w, false).await;
            if r.is_err() {
                println!("{}", r.err().unwrap().to_string());
            }
        });
    }


    #[test]
    pub fn test_update_by_id() {
        async_std::task::block_on(async {
            let rb = init_rbatis().await;
            let activity = BizActivity {
                id: Some("12312".to_string()),
                name: None,
                pc_link: None,
                h5_link: None,
                pc_banner_img: None,
                h5_banner_img: None,
                sort: None,
                status: Some(1),
                remark: None,
                create_time: Some(NaiveDateTime::now()),
                version: Some(1),
                delete_flag: Some(1),
            };
            let r = rb.update_by_id("", &activity).await;
            if r.is_err() {
                println!("{}", r.err().unwrap().to_string());
            }
        });
    }

    #[test]
    pub fn test_fetch_by_wrapper() {
        async_std::task::block_on(async {
            let rb = init_rbatis().await;
            let w = rb.new_wrapper().eq("id", "12312").check().unwrap();
            let r: Result<Option<BizActivity>, Error> = rb.fetch_by_wrapper("", &w).await;
            println!("is_some:{:?}", r);
            if r.is_err() {
                println!("{}", r.err().unwrap().to_string());
            }
        });
    }


    #[test]
    pub fn test_fetch_page_by_wrapper() {
        async_std::task::block_on(async {
            let rb = init_rbatis().await;
            let w = rb.new_wrapper()
                .eq("delete_flag", 1)
                .check().unwrap();
            let r: Page<BizActivity> = rb.fetch_page_by_wrapper("", &w, &PageRequest::new(1, 20)).await.unwrap();
            println!("{}", serde_json::to_string(&r).unwrap());
        });
    }

    #[test]
    pub fn test_list() {
        async_std::task::block_on(async {
            let rb = init_rbatis().await;
            let r: Vec<BizActivity> = rb.list("").await.unwrap();
            println!("{}", serde_json::to_string(&r).unwrap());
        });
    }
}