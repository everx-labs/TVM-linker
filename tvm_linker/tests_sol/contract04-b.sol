pragma solidity ^0.5.0;

contract MyContract {

	uint m_counter;

	function tvm_logstr(bytes32 logstr) private {}
	// A method to be called from another contract
	
	function remoteMethod(uint value) public {
		tvm_logstr("SendMoney");
		msg.sender.transfer(value);
		m_counter = m_counter + 1;
		return; 
	}
	
}
