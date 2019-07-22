pragma solidity ^0.5.0;

contract IMyContract {
	function getCredit() payable public;
}

contract IMyContractCallback {
	function getCreditCallback(uint64 balance) public;
}

contract RemoteContract is IMyContractCallback {

	uint64 m_credit;
	
	// external methods
	
	function getMyCredit(IMyContract bank) payable public {
		bank.getCredit.value(50000)();
		return;
	}
	
	// interface IMyContractCallback
	
	function getCreditCallback(uint64 balance) public {
		m_credit = balance;
	}
	
}
