use std::io::{Read, Write};
use std::net::{TcpStream,Shutdown};
use std::time::{Duration,Instant};
use super::mqtt_handler::MqttHandle;

#[derive(Clone,Debug)]
pub struct MQTTConfig
{
    ///心跳时间间隔 单位为s，超过keep_alive 1.5倍时间未收到心跳包，则释放此连接
    pub keep_alive:f32,
    pub clear_session:bool,
    pub will_flag:bool
}
#[derive(Debug)]
pub enum MsgType {
    CONNECT,
    Pub,
    Sub,
    Disconnect,
    PINGREQ,
    None,
}
#[derive(Debug)]
pub struct MQTT
{
    stream:TcpStream,
    last_buff_time:Instant,
    pub handles:MqttHandle,
    is_closed:bool,
    pub _is_emtpy_link:bool
}

impl MQTT
{

    pub fn init(stream:TcpStream)->MQTT
    {
        let _= stream.set_read_timeout(Some(Duration::from_millis(5)));
        let conf=MQTTConfig{
            keep_alive:0f32,
            clear_session:true,
            will_flag:false
        };
        let handle=MqttHandle::new(conf);
        let res=MQTT{
            stream:stream,
            last_buff_time:Instant::now(),
            handles:handle,
            is_closed:false,
            _is_emtpy_link:true
        };
        res
    }
    pub fn is_closed(&self)->bool
    {
        self.is_closed
    }
    /// 关闭mqtt连接
    pub fn shutdown(&mut self)->bool
    {
        if let Ok(_) = self.stream.shutdown(Shutdown::Both) {
            self.is_closed=true;
            true
        }
        else {
            false
        }
    }
    /// 处理首次的连接请求，带登陆信息
    /// 处理完成后，将从client中对下位机返回标准的MQTT协议CONNACK包
    /// CONNCET包：
    ///    protocol length (byte2 byte3)，表明协议长度
    ///    Connect Flags （byte8），用于表明一些基础配置 bit: 7(username flag) 6(password flag) 5(will retain) 4+3(Will QoS) 2(Will Flag) 1(Clean Session) 0(Reserved)
    ///    Keep Alive (Byte9 byte10)
    /// CONNACK：(固定1byte)包类型 (固定1byte)包长度 (可变1byte)Connect Acknowledge Flags (可变1byte)连接返回代码
    pub fn handle_connect(&mut self)->[u8;4]
    {
        if let Some(buff) = self.handles.handle_connect() {
            self._is_emtpy_link=false;
            [0x20,0x02,buff[0],buff[1]]
        }
        else {
            [0x20,0x02,0x00,0x00]
        }
    }
    ///判断是否超过keep alive时间间隔
    pub fn if_timeout(&mut self)->bool
    {
        let secs:f32=self.last_buff_time.elapsed().as_secs_f32();
        // println!("               secs :{:?}   keep alive {:?}",secs,1.5*self.handles.config.keep_alive);
        if !self._is_emtpy_link && secs>1.5*self.handles.config.keep_alive
        {
            true
        }
        else {
            false
        }
    }
    ///解析
    pub fn handle_loop_work(&mut self)->Option<MsgType>
    {
        if self.handles.buff.len()>=2{
            // let msg_type=self.handles.buff[0]>>4;
            let msg_type=self.handles.buff[0];
            println!("msg type:{:#02x?}",self.handles.buff);
            match msg_type
            {
                0x10=>Some(MsgType::CONNECT),
                0x82=>Some(MsgType::Sub),
                0x30=>Some(MsgType::Pub),
                0xC0=>Some(MsgType::PINGREQ),
                0xE0=>Some(MsgType::Disconnect),
                _=>None
            }
        }
        else{
            None
        }
        
    }
    /// 从tcpstream中读取buff。首先尝试获取前两个字节，用来判断buff的整体长度。
    /// 之后读取所有的buff
    pub fn read_buff(&mut self)->bool
    {
        // let set_res=self.stream.read_timeout();
        // println!("read time out set status :{:#?}",set_res);
        self.handles.buff.clear();
        let mut buff_first=[0u8;13];
        loop {
            if let Ok(size) = self.stream.read(&mut buff_first)  {
                self.handles.buff.extend_from_slice(&buff_first[0..size]);
                // println!("buff read status {:?} <> {:?}",size,buff_first.len());
                if size<buff_first.len()
                {
                    break;
                }
            }
            else {
                break;
            } 
        }
        if self.handles.buff.len()>=2
        {
            self.last_buff_time=Instant::now();
            true
        }
        else {
            false
        }
    }
    pub fn get_first_buff(&mut self)->u8{
        if self.handles.buff.len()>0
        {
            self.handles.buff[0]
        }
        else {
            0
        }
    }
    pub fn write(&mut self,buff:&[u8])->bool
    {
        if let Ok(usize)=self.stream.write(buff)
        {
            if usize==buff.len()
            {
                self.last_buff_time=Instant::now();
                let fr= self.stream.flush();
                println!("flush result {:?}",fr);
                true
            }
            else {
                false
            }
        }
        else {
            false
        }
    }
    /// 处理心跳事件
    pub fn handle_heartbeat(&mut self)->Option<[u8;2]>
    {
        self.last_buff_time=Instant::now();
        self.handles.handle_heartbeat()
    }
}