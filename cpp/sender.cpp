#include "rtc/rtc.hpp"
#include <string>
using std::shared_ptr;

int main()
{
	

	rtc::InitLogger(rtc::LogLevel::Warning);

    rtc::Configuration config;
    config.iceServers.emplace_back("stun.l.google.com:19302");

    auto pc = std::make_shared<rtc::PeerConnection>(config);
	pc->onStateChange([](rtc::PeerConnection::State state)
                      { std::cout << "[State: " << state << "]" << std::endl; });

    pc->onGatheringStateChange([&](rtc::PeerConnection::GatheringState state)
                               {
        std::cout << "[Gathering State: " << state << "]" << std::endl;
        if (state == rtc::PeerConnection::GatheringState::Complete) {
            auto description = pc->localDescription().value();
            std::cout << "[Complete SDP: " << description << "]" << std::endl;
			std::cout << "Enter the remote SDP:";


        }; });

	auto dc = pc->createDataChannel("test");
	dc->onOpen([&]()
               { std::cout << "[DataChannel open: " << dc->label() << "]" << std::endl; });

    dc->onClosed(
        [&]()
        { std::cout << "[DataChannel closed: " << dc->label() << "]" << std::endl; });

    dc->onMessage([](auto data)
                  {
		if (std::holds_alternative<std::string>(data)) {
			std::cout << "[Received: " << std::get<std::string>(data) << "]" << std::endl;
		} });
	
	//
	//std::cout << "Enter the remote SDP: ";
	std::string sdp, line;
	while (getline(std::cin, line) && !line.empty())
	{
	 	sdp += line;
	 	sdp += "\r\n";
	}
	
	pc->setRemoteDescription(sdp);

	while (true)
	{
		std::cout << "Enter a message to send: ";
		std::string message;
		std::cin >> message;
		dc->send(message);
	}

}



