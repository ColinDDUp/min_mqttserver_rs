use std::{net::{TcpStream, TcpListener}, thread, sync::mpsc::{Receiver, Sender},time::Duration};
use std::sync::{Mutex, Arc};
use crate::mqtt_entities::mqtt::{MQTT, MsgType};
use std::sync::mpsc::channel;
use super::mqtt_routes::route_entity::RouteEntity;
pub struct MqttServer
{
    _server:TcpListener,
}
impl MqttServer {
    pub fn binding()->MqttServer
    {
        let listener= TcpListener::bind("0.0.0.0:1658").unwrap();
        MqttServer { 
            _server: listener,
        }
    }
    pub fn run(&mut self,server_entity:Arc<Mutex<MqttServerEntity>>)
    {
        let (sender,recv)=channel::<bool>();
        let server_1=Arc::clone(&server_entity);

        thread::spawn(move||start_loop_read(server_1,recv));
        for stream in self._server.incoming()
        {
            let server_x=Arc::clone(&server_entity);
            if let Ok(s) =stream  {
                println!("new client from {:?}",s.peer_addr());
                let send_clone=sender.clone();
                thread::spawn(move||handle_incoming(server_x,s,send_clone));
            }
        }
    }
}
pub struct MqttServerEntity
{
    _routes:RouteEntity,
    _tasks:Vec<Arc<Mutex<MQTT>>>,
    _heart_beat_running:bool,
}
impl MqttServerEntity {
    
    pub fn init()->MqttServerEntity
    {
        MqttServerEntity { 
            _routes: RouteEntity::init(Vec::new(), Vec::new(), Option::None),
            _tasks: Vec::new(), 
            _heart_beat_running:false, 
            
        }
    }   
    
    /// 处理订阅事件
    fn handle_sub(&mut self,client:Arc<Mutex<MQTT>>)
    {
        let client_1=Arc::clone(&client);
        if let Ok(mut client_entity) = client.lock() {
            if !client_entity._is_emtpy_link
            {
                if let Some(routes)= client_entity.handles.handle_subscribe()
                {
                    for route in routes {
                        self._routes.add_route(route, Arc::clone(&client_1))
                    }
                }
            }
        }
        else {
            println!("【error】failed to lock tcp stream xxxx handle sub .");
        }
    }
    /// 处理发布事件
    fn handle_pub(&mut self,client:Arc<Mutex<MQTT>>)
    {
        
        if let Ok(mut client) = client.lock() {
            if !client._is_emtpy_link
            {
                if let Some((route,value)) = client.handles.handle_publish() {
                    let mut client_point:&mut Option<Vec<Arc<Mutex<MQTT>>>>=&mut None;
                    let (if_hit,cp)=self._routes.deep_find_children(route,client_point);
                    client_point=cp;
                    if if_hit
                    {
                        if let Some(cts)=client_point
                        {
                            for ct in cts
                            {
                                if let Ok(mut c)=ct.lock()
                                {
                                    c.write(&value);
                                }
                            }
                        }
                        
                    }
                    // else {
                    //     println!("{:#?}",self._routes);
                    // }
                }    
            }
            
        }
        
    }

    /// 处理心跳
    fn handle_heartbeat(&mut self,client:Arc<Mutex<MQTT>>)
    {
        match client.lock()
        {
            Ok(mut client)=>{
                if !client._is_emtpy_link{
                    if let Some(res)= client.handle_heartbeat()
                    {
                        client.write(&res);
                    }
                }
            },
            Err(e)=>{
                println!("{:?}",e);
            }
        }
    }

    fn handle_connect(&mut self,_client:Arc<Mutex<MQTT>>)
    {
        if let Ok(mut client)=_client.lock()
        {
            let res= client.handle_connect();
            println!("{:02x?}",res);
            client.write(&res);
        }
    }
}

fn handle_incoming(server_client:Arc<Mutex<MqttServerEntity>>, stream:TcpStream,sender:Sender<bool>)
{
    
    let client=MQTT::init(stream);
    println!("将要获取服务模块操作权限");
    let mut server=server_client.lock().unwrap();
    println!("将要获取服务模块操作权限 -- 成功");
    server._tasks.push(Arc::new(Mutex::new(client)));
    if !server._heart_beat_running
    {
        let _=sender.send(true);
    }
}

///循环读取所有mqtt连接的信息，并作出相应的动作
fn start_loop_read(server_client:Arc<Mutex<MqttServerEntity>>,rec:Receiver<bool>)
{
    loop {
        println!("--loop wait for singnal");
        if let Ok(_)=rec.recv()
        {
            println!("--loop start");

            loop {
                if let Ok(mut server)=server_client.lock()
                {
                    let numb=server._tasks.len();
                    if numb<=0
                    {
                        break;
                    }
                    let mut time_out_client_idx=Vec::<usize>::new();

                    for i in 0..numb
                    {
                        let mqtt=Arc::clone(&server._tasks[i]);

                        let mut go_loop=false;
                        if let Ok(mut mq)=mqtt.lock()
                        {
                            go_loop=mq.read_buff();
                            if mq.if_timeout()
                            {
                                println!("{:?} time out!!!!!",i);
                                time_out_client_idx.push(i);
                                mq.shutdown();
                                go_loop=false;
                            }
                        }
                        while go_loop {
                            let mqtt_1=Arc::clone(&server._tasks[i]);
                            let mqtt_2=Arc::clone(&server._tasks[i]);
                            let mut msg_type=MsgType::None;
                            if let Ok(mut mqtt_entity)=mqtt_2.lock()
                            {
                                if mqtt_entity.handles.config.keep_alive>0f32 || mqtt_entity._is_emtpy_link
                                {
                                    if let Some(mt)=mqtt_entity.handle_loop_work()
                                    {
                                        msg_type=mt;
                                    }
                                    else {
                                        msg_type=MsgType::None;
                                    }
                                }
                            };
                            println!("\r\n\r\n>>>>got msg<<<<<\r\n{:#?}\r\n\r\n",msg_type);

                            match msg_type {
                                MsgType::CONNECT=>server.handle_connect(mqtt_1),
                                MsgType::Pub=>server.handle_pub(mqtt_1),
                                MsgType::Sub=>server.handle_sub(mqtt_1),
                                MsgType::Disconnect=>{},
                                MsgType::PINGREQ=>server.handle_heartbeat(mqtt_1),
                                MsgType::None=>{break;},
                            }
                            
                        }
                        
                    }
                    
                    for i in time_out_client_idx
                    {
                        server._tasks.remove(i);
                    }
                }
                
                thread::sleep(Duration::from_millis(500));
            }
            if let Ok(mut server) = server_client.lock() {
                server._heart_beat_running=false;
            }
        }
    }
}
