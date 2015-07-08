use std::io;
use std::net::{Ipv4Addr, SocketAddr};
use std::fmt;

use hyper;
use regex::Regex;

use gateway::Gateway;
use soap;

// Content of the external ip request.
const EXTERNAL_IP_REQUEST: &'static str =
"<SOAP-ENV:Envelope SOAP-ENV:encodingStyle=\"http://schemas.xmlsoap.org/soap/encoding/\" xmlns:SOAP-ENV=\"http://schemas.xmlsoap.org/soap/envelope/\">
    <SOAP-ENV:Body>
        <m:GetExternalIPAddress xmlns:m=\"urn:schemas-upnp-org:service:WANIPConnection:1\">
        </m:GetExternalIPAddress>
    </SOAP-ENV:Body>
</SOAP-ENV:Envelope>";

// Content of the external ip SOAPAction request header.
const GET_EXTERNAL_IP_SOAP_ACTION: &'static str = "\"urn:schemas-upnp-org:service:WANIPConnection:1#GetExternalIPAddress\"";

// Content of the add port mapping SOAPAction request header.
const ADD_PORT_SOAP_ACTION: &'static str = "\"urn:schemas-upnp-org:service:WANIPConnection:1#AddPortMapping\"";

// Content of the delete port mapping SOAPAction request header.
const DELETE_PORT_SOAP_ACTION: &'static str = "\"urn:schemas-upnp-org:service:WANIPConnection:1#DeletePortMapping\"";

// Errors
#[derive(Debug)]
pub enum RequestError {
    HttpError(hyper::Error),
    InvalidResponse,
    IoError(io::Error),
}


impl From<io::Error> for RequestError {
    fn from(err: io::Error) -> RequestError {
        RequestError::IoError(err)
    }
}

impl From<soap::Error> for RequestError {
    fn from(err: soap::Error) -> RequestError {
        match err {
            soap::Error::HttpError(e) => RequestError::HttpError(e),
            soap::Error::IoError(e) => RequestError::IoError(e),
        }
    }
}

#[derive(Debug,Clone,Copy,PartialEq)]
pub enum PortMappingProtocol {
    TCP,
    UDP,
}

impl fmt::Display for PortMappingProtocol {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", match *self {
            PortMappingProtocol::TCP => "TCP",
            PortMappingProtocol::UDP => "UDP",
        })
    }
}

// Get the external IP address.
pub fn get_external_ip(gateway: &Gateway) -> Result<Ipv4Addr, RequestError>  {
    let addr = gateway.addr.clone();
    let url = format!("http://{}:{}{}", addr.ip(), addr.port(),
                      gateway.control_url);
    let text = try!(soap::send(&url, soap::Action::new(GET_EXTERNAL_IP_SOAP_ACTION), EXTERNAL_IP_REQUEST));
    extract_address(&text)
}

// Extract the address from the text.
fn extract_address(text: &str) -> Result<Ipv4Addr, RequestError> {
    let re = Regex::new(r"<NewExternalIPAddress>(\d+\.\d+\.\d+\.\d+)</NewExternalIPAddress>").unwrap();
    match re.captures(text) {
        None => Err(RequestError::InvalidResponse),
        Some(cap) => {
            match cap.at(1) {
                None => Err(RequestError::InvalidResponse),
                Some(ip) => Ok(ip.parse::<Ipv4Addr>().unwrap()),
            }
        },
    }
}

pub fn add_port(gateway: &Gateway, protocol: PortMappingProtocol,
                external_port: u16, local_addr: SocketAddr, lease_duration: u32,
                description: &str) -> Result<(), RequestError> {
    let url = format!("{}", gateway);
    let body = format!("<?xml version=\"1.0\"?>
<s:Envelope xmlns:s=\"http://schemas.xmlsoap.org/soap/envelope/\" s:encodingStyle=\"http://schemas.xmlsoap.org/soap/encoding/\">
<s:Body>
    <u:AddPortMapping xmlns:u=\"urn:schemas-upnp-org:service:WANIPConnection:1\">
        <NewProtocol>{}</NewProtocol>
        <NewExternalPort>{}</NewExternalPort>
        <NewInternalClient>{}</NewInternalClient>
        <NewInternalPort>{}</NewInternalPort>
        <NewLeaseDuration>{}</NewLeaseDuration>
        <NewPortMappingDescription>{}</NewPortMappingDescription>
        <NewEnabled>1</NewEnabled>
        <NewRemoteHost></NewRemoteHost>
    </u:AddPortMapping>
</s:Body>
</s:Envelope>
",
                       protocol, external_port, local_addr.ip(),
                       local_addr.port(), lease_duration, description);
    let text = try!(soap::send(&url, soap::Action::new(ADD_PORT_SOAP_ACTION), &body));
    {
        let re = Regex::new("u:AddPortMappingResponse").unwrap();
        if re.is_match(&text) {
            Ok(())
        } else {
            Err(RequestError::InvalidResponse)
        }
    }
}

pub fn remove_port(gateway: &Gateway, protocol: PortMappingProtocol,
                   external_port: u16) -> Result<(), RequestError> {
    let url = format!("{}", gateway);
    let body = format!("<?xml version=\"1.0\"?>
<s:Envelope xmlns:s=\"http://schemas.xmlsoap.org/soap/envelope/\" s:encodingStyle=\"http://schemas.xmlsoap.org/soap/encoding/\">
  <s:Body>
    <u:DeletePortMapping xmlns:u=\"urn:schemas-upnp-org:service:WANIPConnection:1\">
      <NewProtocol>{}</NewProtocol>
      <NewExternalPort>{}</NewExternalPort>
      <NewRemoteHost>
      </NewRemoteHost>
    </u:DeletePortMapping>
  </s:Body>
</s:Envelope>
", protocol, external_port);
    let text = try!(soap::send(&url, soap::Action::new(DELETE_PORT_SOAP_ACTION), &body));
    {
        let re = Regex::new("u:DeletePortMappingResponse").unwrap();
        if re.is_match(&text) {
            Ok(())
        } else {
            Err(RequestError::InvalidResponse)
        }
    }
}
