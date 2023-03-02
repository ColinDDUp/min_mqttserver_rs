use std::{sync::{Arc,Mutex}};
use super::super::mqtt_entities::mqtt::MQTT;
use std::option::Option;
#[derive(Clone,Debug)]
///mqtt路由实体，包含本机名称，下一级别及可执行的mqtt实体。末级包含可执行的MQTT实体
pub struct RouteEntity
{
    children_name:Vec<String>,
    pub children_entity:Vec<RouteEntity>,
    pub client:Option<Vec<Arc<Mutex<MQTT>>>>
}
impl RouteEntity {
    pub fn init_empty()->RouteEntity
    {
        let r=RouteEntity{
            children_name:Vec::new(),
            children_entity:Vec::new(),
            client:None
        };
        r
    }
    pub fn init(children_name:Vec<String>,children_entity:Vec<RouteEntity>,client:Option<Vec<Arc<Mutex<MQTT>>>>)->RouteEntity
    {
        let r=RouteEntity{
            children_name,
            children_entity:children_entity,
            client:client
        };
        r
    }
    

    pub fn find_child(&mut self,child_route:&str)->Option<(Option<&mut RouteEntity>,&mut Option<Vec<Arc<Mutex<MQTT>>>>)>
    {
        let children_entity=&mut self.children_entity;
        let client=&mut self.client;
        let route_string=child_route.to_string();
        if !route_string.is_empty()
        {
            if let Some(idx)=self.children_name.iter().position(|r|*r==route_string)
            {
                if idx as i16>=0
                {
                    return Some((Some(&mut children_entity[idx]),client));
                }
                
            }
        }
        else
        {
           return Some((None,client)); 
        }
        None
    }
    pub fn get_client(&mut self)->&mut Option<Vec<Arc<Mutex<MQTT>>>>
    {
        &mut self.client
    }
    /// 逐级找到路径对应的MQTT客户端
    /// true 找到
    pub fn deep_find_children<'a>(&'a mut self,route:String,mut _client_point:&'a mut Option<Vec<Arc<Mutex<MQTT>>>>)
    ->(bool,&mut Option<Vec<Arc<Mutex<MQTT>>>>)
    {
        let mut route_names:Vec<&str>=route.split('/').collect();
        route_names.push("");
        let mut entity=self;
        let mut hit=true;
        for nm in route_names
        {
            match entity.find_child(nm)
            {
                Some((r,ct))=>{
                    _client_point=ct;
                    match r {
                        Some(_r)=>entity=_r,
                        None=>{break;}
                    };
                },
                None=>{hit=false;break;},
            }
        }
        (hit,_client_point)
        // hit
    }
    pub fn push_child(&mut self,child_name:String,child_entity:RouteEntity)->&mut RouteEntity
    {
        self.children_name.push(child_name);
        self.children_entity.push(child_entity);
        let idx=self.children_entity.len()-1;
        &mut self.children_entity[idx]
    }
    /// 向第idx个子集中，插入新的mqtt client,并检查之前的是否有已经关闭的。
    /// 如果有关闭的，则删掉
    fn check_and_push(&mut self,idx:usize,client:Arc<Mutex<MQTT>>)
    {
        let  child=&mut self.children_entity[idx].client;
        match child
        {
            Some(vec)=>{
                let mut idx_ary:Vec<usize>=Vec::new();
                for idx in 0..vec.len()
                {
                    if let Ok(mqt)=vec[idx].lock()
                    {
                        if mqt.is_closed()
                        {
                            idx_ary.push(idx);
                        }
                    }
                }
                let mut remove_count=0;
                for idx in idx_ary
                {
                    println!("will remove {:?}",idx-remove_count);
                    vec.remove(idx-remove_count);
                    remove_count+=1;
                }
                vec.push(client);
            },
            None=>{
                *child=Some(vec![client]);
            }
        }
    }
    ///添加新的订阅路由
    pub fn add_route(&mut self,mut routes:Vec<String>,client:Arc<Mutex<MQTT>>)
    {
        let name=routes[0].to_string();
        let mut c_name:Vec<String>=Vec::new();
        c_name.clone_from(&self.children_name);

        if let Some(idx)=c_name.into_iter().position(|x|x==name){
            if routes.len()>1
            {
                routes.remove(0);
                self.children_entity[idx].add_route(routes, client);
            }
            else {
                self.check_and_push(idx,client);
            }
        }
        else {
            let mut route_obj=self;
            for rt in routes
            {
                let route_entity=RouteEntity::init_empty();
                route_obj=route_obj.push_child(rt.to_string(), route_entity);
            }
            route_obj.client=Some(vec![client]);
        }
        
        
    }

}