use std::io::Read;

pub trait ChannelExt {
    fn consume(&self);
}

impl ChannelExt for ssh2::Channel {
    fn consume(&self) {
        let mut buf = String::new();
        self.stream(0).read_to_string(&mut buf).unwrap();
        self.stderr().read_to_string(&mut buf).unwrap();
    }
}
