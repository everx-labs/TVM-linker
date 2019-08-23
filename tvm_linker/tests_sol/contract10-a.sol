pragma solidity ^0.5.0;

import "contract10.sol";

// Tests for sending and receiving arrays

contract Sender {

	uint m_counter;

	function send_uint64(address receiver, uint64 count) public {
		uint64[] memory arr = new uint64[](count);
		for (uint64 i = 0; i < count; i++) {
			arr[i] = i+1;
		}
		IReceiver(receiver).on_uint64(arr);
		m_counter++;
	}
	
}
