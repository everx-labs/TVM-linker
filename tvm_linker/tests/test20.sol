pragma solidity ^0.5.0;

/*
		  EmitStatement
			 Gas costs: 1379
			 Source: "emit TestEvent(a, b)"
			FunctionCall
			   Type: tuple()
			   Source: "TestEvent(a, b)"
			  Identifier TestEvent
				 Type: function (uint32,uint32)
				 Source: "TestEvent"
			  Identifier a
				 Type: uint32
				 Source: "a"
			  Identifier b
				 Type: uint32
				 Source: "b"
*/

contract Test20 {

	// testing events
	
	event TestEvent(
		uint32 aaa,
		uint32 bbb
	);

	function test19(uint32 a, uint32 b) public {
		emit TestEvent(a, b);
		return;
	}
}