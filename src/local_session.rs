use log::{error, info, warn};
use rdbc::Connection;
use serde::de;
use serde_json::de::ParserNumber;
use serde_json::Value;
use uuid::Uuid;

use crate::decode::rdbc_driver_decoder::decode_result_set;
use crate::queryable::Queryable;
use crate::session::Session;
use crate::tx::propagation::Propagation;
use crate::tx::save_point_stack::SavePointStack;
use crate::tx::tx::Tx;
use crate::tx::tx_stack::TxStack;
use crate::utils::{driver_util, rdbc_util};
use crate::utils::rdbc_util::to_rdbc_values;

pub struct LocalSession<'a> {
    pub session_id: String,
    pub driver: String,
    pub tx_stack: TxStack<'a>,
    pub save_point_stack: SavePointStack,
    pub is_closed: bool,
    pub new_local_session: Option<Box<LocalSession<'a>>>,
    pub enable_log: bool,
    pub conn: Option<Box<dyn Connection>>,
}

impl<'a> LocalSession<'a> {
    pub fn new(id: &str, driver: &str, conn_opt: Option<Box<dyn Connection>>) -> Result<Self, String> {
        let mut new_id = id.to_string();
        if new_id.is_empty() {
            new_id = Uuid::new_v4().to_string();
        }
        let mut conn = conn_opt;
        if conn.is_none() {
            let r = driver_util::get_conn_by_link(driver)?;
            conn = Some(r);
        }
        return Ok(Self {
            session_id: new_id,
            driver: driver.to_string(),
            tx_stack: TxStack::new(),
            save_point_stack: SavePointStack::new(),
            is_closed: false,
            new_local_session: None,
            enable_log: true,
            conn: conn,
        });
    }
}

impl<'a> Session<'a> for LocalSession<'a> {
    fn id(&self) -> String {
        return Uuid::new_v4().to_string();
    }

    fn query<T>(&mut self, sql: &str, arg_array: &mut Vec<Value>) -> Result<T, String> where T: de::DeserializeOwned {
        if self.is_closed == true {
            return Err("[rbatis] session can not query a closed session!".to_string());
        }
        if self.new_local_session.is_some() {
            return self.new_local_session.as_mut().unwrap().query(sql, arg_array);
        }
        let params = to_rdbc_values(arg_array);
        if self.enable_log {
            info!("[rbatis] Query: ==>  {}: ", sql);
            info!("[rbatis]  Args: ==>  {}: ", rdbc_util::rdbc_vec_to_string(&params));
        }
        let (mut t_opt, _) = self.tx_stack.last_pop();
        if t_opt.is_some() {
            let mut t = t_opt.unwrap();
            let result = t.query(sql, arg_array)?;
            return result;
        } else {
            return self.conn.as_mut().unwrap().query(self.enable_log, sql, &params);
        }
    }

    fn exec(&mut self, sql: &str, arg_array: &mut Vec<Value>) -> Result<u64, String> {
        if self.is_closed == true {
            return Err("[rbatis] session can not query a closed session!".to_string());
        }
        if self.new_local_session.is_some() {
            return self.new_local_session.as_mut().unwrap().query(sql, arg_array);
        }
        let params = to_rdbc_values(arg_array);
        if self.enable_log {
            info!("[rbatis] Query: ==>  {}: ", sql);
            info!("[rbatis]  Args: ==>  {}: ", rdbc_util::rdbc_vec_to_string(&params));
        }
        let (mut t_opt, _) = self.tx_stack.last_pop();
        if t_opt.is_some() {
            let mut t = t_opt.unwrap();
            let result = t.exec(sql, arg_array)?;
            return Ok(result);
        } else {
            return self.conn.as_mut().unwrap().exec(self.enable_log, sql, &params);
        }
    }

    fn rollback(&mut self) -> Result<u64, String> {
        if self.is_closed == true {
            return Err("[rbatis] session can not query a closed session!".to_string());
        }
        let mut closec_num = 0;
        if self.new_local_session.is_some() {
            let new_session = self.new_local_session.as_mut().unwrap();
            let r = new_session.rollback()?;
            new_session.close();
            closec_num += r;
        }

        let (t_opt, p_opt) = self.tx_stack.pop();
        if t_opt.is_some() && p_opt.is_some() {
            let mut t = t_opt.unwrap();
            if self.last_propagation().is_some() {
                if self.last_propagation().as_ref().unwrap().eq(&Propagation::NESTED) {
                    let point_opt = self.save_point_stack.pop();
                    if point_opt.is_some() {
                        info!("[rbatis] [{}] exec ============ rollback", self.session_id.as_str());
                        let sql = "rollback to ".to_string() + point_opt.unwrap().as_str();
                        let r = t.exec(sql.as_str(), &mut vec![])?;
                        closec_num += r;
                    }
                }
            }
            if self.tx_stack.len() == 0 {
                info!("[rbatis] [{}] exec ============ rollback", self.session_id.as_str());
                let r = t.rollback()?;
                closec_num += r;
            }
        }
        return Ok(closec_num);
    }

    fn commit(&mut self) -> Result<u64, String> {
        if self.is_closed == true {
            return Err("[rbatis] session can not query a closed session!".to_string());
        }
        let mut closec_num = 0;
        if self.new_local_session.is_some() {
            let new_session = self.new_local_session.as_mut().unwrap();
            let r = new_session.rollback()?;
            new_session.close();
            closec_num += r;
        }
        let (t_opt, p_opt) = self.tx_stack.pop();
        if t_opt.is_some() && p_opt.is_some() {
            let mut t = t_opt.unwrap();
            if self.last_propagation().is_some() {
                if self.last_propagation().as_ref().unwrap().eq(&Propagation::NESTED) {
                    let p_id = format!("p{}", self.tx_stack.len() + 1);
                    self.save_point_stack.push(p_id.as_str());
                    let sql = format!("savepoint {}", p_id.as_str());
                    let r = t.exec(sql.as_str(), &mut vec![])?;
                    closec_num += r;
                }
            }
            if self.tx_stack.len() == 0 {
                info!("[rbatis] [{}] exec ============ rollback", self.session_id.as_str());
                let r = t.commit()?;
                closec_num += r;
            }
        }
        return Ok(closec_num);
    }

    fn begin(&'a mut self, propagation_type: Option<Propagation>) -> Result<u64, String> {
        if propagation_type.is_some() {
            match propagation_type.as_ref().unwrap() {
                ///默认，表示如果当前事务存在，则支持当前事务。否则，会启动一个新的事务。have tx ? join : new tx()
                Propagation::REQUIRED => {
                    if self.tx_stack.len() > 0 {
                        let (l_t, l_p) = self.tx_stack.last_pop();
                        if l_t.is_some() && l_p.is_some() {
                            self.tx_stack.push(l_t.unwrap(), l_p.unwrap());
                        }
                    } else {
                        //new tx
                        let tx = Tx::begin("", self.driver.as_str(), self.enable_log, self.conn.as_mut())?;
                        self.tx_stack.push(tx, propagation_type.unwrap());
                    }
                }
                Propagation::SUPPORTS => {
                    return Ok(0);
                }
                Propagation::MANDATORY => {
                    if self.tx_stack.len() > 0 {
                        return Ok(0);
                    } else {
                        return Err("[rbatis] PROPAGATION_MANDATORY Nested transaction exception! current not have a transaction!".to_string());
                    }
                }
                Propagation::REQUIRES_NEW => {
                    if self.tx_stack.len() > 0 {
                        //TODO stop old tx
                    }
                    //new session
                    let r = driver_util::get_conn_by_link(self.driver.as_str());
                    if r.is_err() {
                        return Err(r.err().unwrap());
                    }
                    let new_session = LocalSession::new("", self.driver.as_str(), Option::from(r.unwrap()))?;
                    self.new_local_session = Some(Box::new(new_session));
                }
                Propagation::NOT_SUPPORTED => {
                    if self.tx_stack.len() > 0 {
                        //TODO stop old tx
                    }
                    let r = driver_util::get_conn_by_link(self.driver.as_str());
                    if r.is_err() {
                        return Err(r.err().unwrap());
                    }
                    let new_session = LocalSession::new("", self.driver.as_str(), Option::from(r.unwrap()))?;
                    self.new_local_session = Some(Box::new(new_session));
                }
                Propagation::NEVER => {
                    if self.tx_stack.len() > 0 {
                        return Err("[rbatis] PROPAGATION_NEVER  Nested transaction exception! current Already have a transaction!".to_string());
                    }
                }
                ///表示如果当前事务存在，则在嵌套事务内执行，如嵌套事务回滚，则只会在嵌套事务内回滚，不会影响当前事务。如果当前没有事务，则进行与PROPAGATION_REQUIRED类似的操作。
                Propagation::NESTED => {
                    if self.tx_stack.len() > 0 {
                        let (l_t, l_p) = self.tx_stack.last_pop();
                        if l_t.is_some() && l_p.is_some() {
                            self.tx_stack.push(l_t.unwrap(), l_p.unwrap());
                        }
                    } else {
                        return self.begin(Option::Some(Propagation::REQUIRED));
                    }
                }
                Propagation::NOT_REQUIRED => {
                    if self.tx_stack.len() > 0 {
                        return Err("[rbatis] PROPAGATION_NOT_REQUIRED Nested transaction exception! current Already have a transaction!".to_string());
                    } else {
                        //new tx
                        let tx = Tx::begin("", self.driver.as_str(), self.enable_log, self.conn.as_mut())?;
                        self.tx_stack.push(tx, propagation_type.unwrap());
                    }
                }
                _ => {
                    return Err("[rbatis] Nested transaction exception! not support PROPAGATION in begin!".to_string());
                }
            }
        }
        return Ok(0);
    }

    fn close(&mut self) {
        if self.is_closed {
            return;
        }
        self.is_closed = true;
    }

    fn last_propagation(&self) -> Option<Propagation> {
        if self.tx_stack.len() != 0 {
            let (tx_opt, prop_opt) = self.tx_stack.last_ref();
            if prop_opt.is_some() {
                return Some(prop_opt.unwrap().clone());
            }
        }
        return None;
    }
}
