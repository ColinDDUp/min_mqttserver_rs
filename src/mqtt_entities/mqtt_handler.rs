use super::mqtt::MQTTConfig;
#[derive(Debug)]
///处理心跳信息
pub struct MqttHandle
{
    pub buff:Vec<u8>,
    pub config:MQTTConfig
}
impl MqttHandle {
    pub fn new(conf:MQTTConfig)->MqttHandle
    {
        MqttHandle{
            buff:Vec::new(),
            config:conf
        }
    }
    pub fn judge_if_buff_legal(&mut self)->(bool,usize)
    {
        println!("self.buff.len() {:?}",self.buff.len());
        if self.buff.len()<2
        {
            return (false,0);
        }
        let remaining_length:usize=self.buff[1].into();
        println!("remain {:?}",remaining_length);
        if self.buff.len()< remaining_length+2
        {
            return (false,remaining_length);
        }
        (true,remaining_length)
    }
    pub fn handle_connect(&mut self)->Option<[u8;2]> {
        let (goon,remaining_length)=self.judge_if_buff_legal();
        if !goon {
            return None
        }
        let play_load:Vec<u8>=self.buff.drain(..remaining_length+2).collect();
        let (go_on,res)=self.set_configs(play_load);
        if !go_on
        {
            None
        }
        else {
            Some(res)
        }
    }
    /// 处理发布，提取发布的路径及发布值
    /// return (路径，值)
    pub fn handle_publish(&mut self)->Option<(String,Vec<u8>)>
    {
        let (goon,remaining_length)=self.judge_if_buff_legal();
        if !goon {
            return None
        }
        let play_load=&self.buff[..remaining_length+2];
        let topic_length:usize=play_load[3].into();
        let mut res:Option<(String,Vec<u8>)>=None;
        if let Ok(topic_name)=String::from_utf8(play_load[4..topic_length+4].to_vec()){
            println!("===publish==topic:{:?}",topic_name);
            res= Some((topic_name,play_load.to_vec()));
        }
        self.buff.drain(..remaining_length+2);
        res
        // Some((String::new(),Vec::<u8>::new()))
    }
    pub fn handle_heartbeat(&mut self)->Option<[u8;2]>
    {
        let (goon,remaining_length)= self.judge_if_buff_legal();
        if !goon
        {
            return None;
        }
        self.buff.drain(..remaining_length+2);
        Some([0xD0, 0x00])
    }
    ///处理订阅，提取出订阅的路径，可以获取多个订阅路径
    pub fn handle_subscribe(&mut self)->Option<Vec<Vec<String>>>
    {
        // println!("【sub】\r\n{:#02x?}",self.buff);
        let (goon,remaining_length)=self.judge_if_buff_legal();
        if !goon {
            return None
        }
        //开始取值时，空出可变报头的2 byte
        let mut last_length=2;
        let buff=&mut self.buff;
        let mut play_load:Vec<u8>=buff.drain(..remaining_length+2).collect();
        play_load.remove(0);
        play_load.remove(0);
        let mut idx=0;
        let mut res=Vec::<Vec<String>>::new();
        loop {
            if idx+last_length<play_load.len() {
                let lg:usize=(play_load[last_length+1]).into();
                // last_length+=2 跨过主体过滤器的前两个byte 即 MSG length 和LSB length
                last_length+=2;
                if let Ok(msg)=String::from_utf8(play_load[last_length..lg+last_length].to_vec())
                {
                    println!("***sub***topic:{:?}",msg);
                    let rr:Vec<&str>= msg.split('/').collect();
                    let mut rs:Vec<String>=Vec::new();
                    for r in rr
                    {
                        rs.push(r.to_string());
                    }
                    res.push(rs);
                    last_length+=lg;
                }
                else {
                    break;
                }
            }
            else {
                break;
            }
            idx+=1;
        }
        Some(res)
    }
    /// 设置MQTT基础配置，并根据配置组织返回数据
    fn set_configs(&mut self,buff:Vec<u8>) ->(bool,[u8;2])
    {
        // println!("===set config==\r\n{:#02x?}",self.buff);
        if buff.len()<4
        {
            return (false,[0x00,0x00]);
        }
        let protocol_length:usize=u16::from_le_bytes([buff[3],buff[2]]).into();
        // println!("===set config==protocle_length==\r\n{:?} (2){:#02x?} (3){:#02x?}",protocol_length,self.buff[2],self.buff[3]);
        if buff.len()<7+protocol_length
        {
            return (false,[0x00,0x00]);
        }
        let protocol_flag:u8=buff[5+protocol_length];
        let has_usname=0b10000000 & protocol_flag == 0b10000000;
        if !has_usname
        {
            return (false,[0x00,0x00]);
        }
        let has_password=0b01000000 & protocol_flag == 0b01000000;
        if !has_password
        {
            return (false,[0x00,0x00]);
        }
        self.config.clear_session= 0b00000010 & protocol_flag == 0b00000010;
        //是否在断开时发送遗嘱，如果为true，则payload中必须包含will topic和will message字段
        self.config.will_flag = 0b00000100 & protocol_flag == 0b00000100;
        if protocol_flag & 0b00000001 != 0
        {
            return (false,[0x00,0x00]);
        }
        self.config.keep_alive=u16::from_le_bytes([buff[ 7+ protocol_length],buff[6+protocol_length]]).into();
        println!("===set config==\r\n{:#?}",self.config);
        (true,if self.config.clear_session {[0x00,0x00]} else{[0x01,0x00]})
    }
    
   
}
